use crate::rendering::config::{BoundingBoxMode, RenderingConfig};
use crate::rendering::devtools::DevToolsGUI;
use crate::rendering::event::{LightTextureType, RenderingEvent};
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::primitive::circle_lines::Circle3DLines;
use crate::rendering::primitive::cube_lines::Cube3DLines;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::primitive::segment_lines::Segment3DLines;
use crate::rendering::shaders::billboard::BillboardShader;
use crate::rendering::shaders::line::LineShader;
use crate::rendering::shaders::{BILLBOARD_SHADER, LINE_SHADER};
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::framebuffer::{
    BlitFramebufferFilter, BlitFramebufferMask, Framebuffer,
};
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{Mat4, Quat, UVec2, Vec2, Vec3, Vec4};
use glow::HasContext;
use std::cell::RefCell;
use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

pub(crate) struct DevtoolsPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,
    gui: Rc<RefCell<DevToolsGUI>>,

    // Resources
    line_shader: Option<LineShader>,
    billboard_shader: Option<BillboardShader>,
    sun_light_texture: Option<TypedAsset<Texture>>,
    point_light_texture: Option<TypedAsset<Texture>>,

    // Primitives
    cube: Cube3DLines,
    quad: Quad2D,
    segment: Segment3DLines,
    circle: Circle3DLines,

    // Runtime variables
    viewport_size: UVec2,
    sunlight_distance: f32,
    view: Mat4,
    gbuffer: Rc<GBuffer>,
}

enum ProcessResult {
    Skipped,
    Rendered(RenderResult),
    RenderWithDepthBlit(RenderResult),
}

impl DevtoolsPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        config: RenderingConfig,
        gui: Rc<RefCell<DevToolsGUI>>,
    ) -> Self {
        DevtoolsPass {
            gl: gl.clone(),
            id,
            line_shader: None,
            billboard_shader: None,
            sun_light_texture: None,
            point_light_texture: None,
            cube: Cube3DLines::new(gl.clone()),
            quad: Quad2D::new(gl.clone()),
            segment: Segment3DLines::new(gl.clone()),
            circle: Circle3DLines::new(gl.clone()),
            viewport_size: UVec2::ZERO,
            sunlight_distance: 0.0,
            view: Default::default(),
            gbuffer,
            config,
            gui,
        }
    }

    fn draw_axis_helper(&self) -> RenderResult {
        static X_COLOR: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        static Y_COLOR: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);
        static Z_COLOR: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

        static LENGTH: f32 = 0.1; // In world space
        static DISTANCE: f32 = 1.5; // In front of camera

        let (_, camera_rotation, camera_translation) =
            self.view.inverse().to_scale_rotation_translation();
        let forward = camera_rotation * -Vec3::Z;
        let camera_position = camera_translation + forward * DISTANCE;
        let scale = Vec3::splat(LENGTH);

        let x_model = Mat4::from_scale_rotation_translation(
            scale,
            Quat::from_rotation_arc(Vec3::Z, Vec3::X),
            camera_position,
        );

        let y_model = Mat4::from_scale_rotation_translation(
            scale,
            Quat::from_rotation_arc(Vec3::Z, Vec3::Y),
            camera_position,
        );

        let z_model = Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, camera_position);

        let shader = self.line_shader.as_ref().unwrap();
        let program = shader.asset.cast();

        let mut result = RenderResult::default();
        program.set_uniform(&shader.color_location, X_COLOR);
        program.set_uniform(&shader.model_location, x_model);
        result += self.segment.draw();
        program.set_uniform(&shader.color_location, Y_COLOR);
        program.set_uniform(&shader.model_location, y_model);
        result += self.segment.draw();
        program.set_uniform(&shader.color_location, Z_COLOR);
        program.set_uniform(&shader.model_location, z_model);
        result += self.segment.draw();

        result
    }

    fn draw_point_light_billboards(&self, frame: &DataStreamFrame) -> RenderResult {
        let shader = self.billboard_shader.as_ref().unwrap();
        let program = shader.asset.cast();

        let tex = self.point_light_texture.as_ref().unwrap().cast();
        Texture::bind(&self.gl, TextureBind::Texture2D, tex, 0);
        program.set_uniform(&shader.size_location, Vec2::new(0.3, 0.3));

        let mut result = RenderResult::default();

        for point_light in frame.point_lights.iter() {
            let position = point_light.position;
            program.set_uniform(&shader.position_location, position);
            result += self.quad.draw();
        }

        result
    }

    fn draw_sun_light_billboards(&self, frame: &DataStreamFrame) -> RenderResult {
        let shader = self.billboard_shader.as_ref().unwrap();
        let program = shader.asset.cast();

        let tex = self.sun_light_texture.as_ref().unwrap().cast();
        Texture::bind(&self.gl, TextureBind::Texture2D, tex, 0);

        let mut result = RenderResult::default();
        program.set_uniform(&shader.size_location, Vec2::new(2.0, 2.0));

        for sun_light in frame.sun_lights.iter() {
            let position = -sun_light.direction.normalize() * self.sunlight_distance; // Position it far away in the light direction
            program.set_uniform(&shader.position_location, position);
            result += self.quad.draw();
        }

        result
    }

    fn draw_point_light_lines(&self, frame: &DataStreamFrame) -> RenderResult {
        static LINE_COLOR: Vec4 = Vec4::new(1.0, 1.0, 0.0, 1.0);

        let shader = self.line_shader.as_ref().unwrap();
        let program = shader.asset.cast();

        program.set_uniform(&shader.color_location, LINE_COLOR);

        // Draw 3 circles for each point light to represent the light's range
        let mut result = RenderResult::default();
        for point_light in frame.point_lights.iter() {
            let position = point_light.position;

            let range = if point_light.linear_falloff {
                point_light.range * 0.5
            } else {
                point_light.range * 0.5 * 0.5
            };
            let scale = Mat4::from_scale(Vec3::splat(range));

            let model1 = Mat4::from_rotation_translation(
                Quat::from_axis_angle(Vec3::X, FRAC_PI_2),
                position,
            ) * scale;
            let model2 = Mat4::from_rotation_translation(
                Quat::from_axis_angle(Vec3::Y, FRAC_PI_2),
                position,
            ) * scale;
            let model3 = Mat4::from_rotation_translation(
                Quat::from_axis_angle(Vec3::Z, FRAC_PI_2),
                position,
            ) * scale;

            program.set_uniform(&shader.model_location, model1);
            result += self.circle.draw();
            program.set_uniform(&shader.model_location, model2);
            result += self.circle.draw();
            program.set_uniform(&shader.model_location, model3);
            result += self.circle.draw();
        }

        result
    }

    fn draw_sun_light_gizmos(&self, frame: &DataStreamFrame) -> RenderResult {
        static LINE_COLOR: Vec4 = Vec4::new(0.3, 0.7, 0.9, 1.0);

        let shader = self.line_shader.as_ref().unwrap();
        let program = shader.asset.cast();

        program.set_uniform(&shader.color_location, LINE_COLOR);

        // Draw very long line to represent the sunlight's direction
        let mut result = RenderResult::default();
        for sun_light in frame.sun_lights.iter() {
            let direction = sun_light.direction.normalize();

            // Segment is a 1-unit long line along Z-axis
            let model = Mat4::from_rotation_translation(
                Quat::from_rotation_arc(Vec3::Z, direction),
                -direction * self.sunlight_distance,
            ) * Mat4::from_scale(Vec3::splat(self.sunlight_distance * 2.0));

            program.set_uniform(&shader.model_location, model);
            result += self.segment.draw();
        }

        result
    }

    fn process_gizmos(&mut self, frame: &DataStreamFrame, blit: bool) -> ProcessResult {
        if self.billboard_shader.is_none() {
            return ProcessResult::Skipped;
        }
        if self.line_shader.is_none() {
            return ProcessResult::Skipped;
        }
        if self.sun_light_texture.is_none() || self.point_light_texture.is_none() {
            return ProcessResult::Skipped;
        }
        if !self.config.get_show_gizmos() {
            return ProcessResult::Skipped;
        }

        let mut blit = blit;
        if !blit {
            // Blit the depth buffer to the default framebuffer
            Framebuffer::blit_to_default(
                &self.gl,
                &self.gbuffer.fbo,
                self.viewport_size,
                BlitFramebufferMask::Depth,
                BlitFramebufferFilter::Nearest,
            );

            // Enable depth test
            unsafe {
                self.gl.enable(glow::DEPTH_TEST);
                self.gl.depth_func(glow::LEQUAL);
            }
            blit = true;
        }

        unsafe {
            // Enable blending for transparency
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.line_width(2.0);
        }

        let mut result = RenderResult::default();

        let shader = self.line_shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        result += self.draw_point_light_lines(frame);
        result += self.draw_sun_light_gizmos(frame);

        unsafe {
            // Make the axis at top of all geometry
            self.gl.disable(glow::DEPTH_TEST);
        }
        result += self.draw_axis_helper();

        Program::unbind(&self.gl);

        unsafe {
            // Make the axis at top of all geometry
            self.gl.enable(glow::DEPTH_TEST);
        }

        let shader = self.billboard_shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);
        result += self.draw_point_light_billboards(frame);
        result += self.draw_sun_light_billboards(frame);
        Program::unbind(&self.gl);

        if blit {
            ProcessResult::RenderWithDepthBlit(result)
        } else {
            ProcessResult::Rendered(result)
        }
    }

    fn process_bounding_boxes(&mut self, frame: &DataStreamFrame, blit: bool) -> ProcessResult {
        if self.line_shader.is_none() {
            return ProcessResult::Skipped;
        }

        let mut blit = blit;
        match self.config.get_bounding_box_mode() {
            BoundingBoxMode::Disabled => return ProcessResult::Skipped,
            BoundingBoxMode::AABBHonorDepth | BoundingBoxMode::OBBHonorDepth if !blit => {
                // Blit the depth buffer to the default framebuffer
                Framebuffer::blit_to_default(
                    &self.gl,
                    &self.gbuffer.fbo,
                    self.viewport_size,
                    BlitFramebufferMask::Depth,
                    BlitFramebufferFilter::Nearest,
                );

                // Enable depth test
                unsafe {
                    self.gl.enable(glow::DEPTH_TEST);
                    self.gl.depth_func(glow::LEQUAL);
                }
                blit = true;
            }
            _ => {}
        }

        // Bind shader
        let shader = self.line_shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        let mut result = RenderResult::default();
        for renderable in frame.renderables.iter() {
            let mesh = renderable.mesh.cast();

            static MESH_COLOR: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
            static SUBMESH_COLOR: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);

            fn draw_cube(
                pass: &DevtoolsPass,
                renderable_model: Mat4,
                min: Vec3,
                max: Vec3,
            ) -> RenderResult {
                let shader = pass.line_shader.as_ref().unwrap();
                let program = shader.asset.cast();
                let mode = pass.config.get_bounding_box_mode();

                match mode {
                    BoundingBoxMode::OBB | BoundingBoxMode::OBBHonorDepth => pass.cube.draw(
                        |model| {
                            let obb = renderable_model * model;
                            program.set_uniform(&shader.model_location, obb);
                        },
                        min,
                        max,
                    ),
                    BoundingBoxMode::AABB | BoundingBoxMode::AABBHonorDepth => {
                        let (min, max) = FrustumCulling::obb_to_aabb(min, max, renderable_model);
                        pass.cube.draw(
                            |model| {
                                program.set_uniform(&shader.model_location, model);
                            },
                            min,
                            max,
                        )
                    }
                    _ => unreachable!(),
                }
            }

            program.set_uniform(&shader.color_location, MESH_COLOR);
            result += draw_cube(self, renderable.model, mesh.min, mesh.max);

            program.set_uniform(&shader.color_location, SUBMESH_COLOR);
            for bucket in &mesh.buckets {
                for submesh in &bucket.submesh {
                    result += draw_cube(self, renderable.model, submesh.min, submesh.max);
                }
            }
        }

        Program::unbind(&self.gl);

        if blit {
            ProcessResult::RenderWithDepthBlit(result)
        } else {
            ProcessResult::Rendered(result)
        }
    }

    fn process_overlays(
        &mut self,
        win: &Window,
        backend: &RendererBackend<RenderingEvent>,
    ) -> ProcessResult {
        ProcessResult::Rendered(self.gui.borrow_mut().render(win, backend))
    }
}

impl RenderPass<RenderingEvent> for DevtoolsPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_bounding_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut DevtoolsPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_bounding_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.line_shader = None;
                self.billboard_shader = None;
                self.sun_light_texture = None;
                self.point_light_texture = None;
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = view;
            }
            RenderingEvent::ViewportResized(size) => {
                self.viewport_size = size;
            }
            RenderingEvent::PerspectiveProjectionUpdated(_, _, far) => {
                self.sunlight_distance = far * 0.9;
            }

            RenderingEvent::UpdateShader(name, shader) if name == BILLBOARD_SHADER.into() => {
                self.billboard_shader = Some(BillboardShader::new(shader.clone()).unwrap());

                let shader = self.billboard_shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.texture_location, 0);
                Program::unbind(&self.gl);
            }

            RenderingEvent::UpdateShader(name, shader) if name == LINE_SHADER.into() => {
                self.line_shader = Some(LineShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.line_shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                Program::unbind(&self.gl);
            }

            RenderingEvent::SetLightTexture(kind, texture) => match kind {
                LightTextureType::SunLight => {
                    self.sun_light_texture = Some(texture);
                }
                LightTextureType::PointLight => {
                    self.point_light_texture = Some(texture);
                }
            },

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "DevtoolsPass"
    }

    fn begin(
        &mut self,
        win: &Window,
        backend: &RendererBackend<RenderingEvent>,
        frame: &DataStreamFrame,
    ) -> RenderResult {
        let mut result = RenderResult::default();
        let mut blit = false;

        result += match self.process_gizmos(frame, blit) {
            ProcessResult::Skipped => RenderResult::default(),
            ProcessResult::Rendered(r) => r,
            ProcessResult::RenderWithDepthBlit(r) => {
                blit = true;
                r
            }
        };
        result += match self.process_bounding_boxes(frame, blit) {
            ProcessResult::Skipped => RenderResult::default(),
            ProcessResult::Rendered(r) => r,
            ProcessResult::RenderWithDepthBlit(r) => {
                blit = true;
                r
            }
        };
        result += match self.process_overlays(win, backend) {
            ProcessResult::Skipped => RenderResult::default(),
            ProcessResult::Rendered(r) => r,
            ProcessResult::RenderWithDepthBlit(r) => {
                blit = true;
                r
            }
        };

        result
    }
}
