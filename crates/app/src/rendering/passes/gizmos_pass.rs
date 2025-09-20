use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::primitive::circle_lines::Circle3DLines;
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
use std::f32::consts::FRAC_PI_2;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct GizmosPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,

    billboard: Option<BillboardShader>,
    line: Option<LineShader>,

    viewport_size: UVec2,
    quad: Quad2D,
    segment: Segment3DLines,
    circle: Circle3DLines,

    light_texture: Option<TypedAsset<Texture>>,

    gbuffer: Rc<GBuffer>,
    config: RenderingConfig,
}

impl GizmosPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        config: RenderingConfig,
    ) -> Self {
        GizmosPass {
            gl: gl.clone(),
            id,
            billboard: None,
            line: None,
            viewport_size: Default::default(),
            quad: Quad2D::new(gl.clone()),
            segment: Segment3DLines::new(gl.clone()),
            circle: Circle3DLines::new(gl.clone()),
            light_texture: None,
            gbuffer,
            config,
        }
    }

    fn draw_point_light_billboards(&self, frame: &DataStreamFrame) -> RenderResult {
        let shader = self.billboard.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        let light_texture = self.light_texture.as_ref().unwrap().cast();
        Texture::bind(&self.gl, TextureBind::Texture2D, light_texture, 0);

        let mut result = RenderResult::default();

        for point_light in frame.point_lights.iter() {
            let position = point_light.position;
            program.set_uniform(&shader.position_location, position);
            result += self.quad.draw();
        }
        Program::unbind(&self.gl);

        result
    }

    fn draw_point_light_lines(&self, frame: &DataStreamFrame) -> RenderResult {
        static LINE_COLOR: Vec4 = Vec4::new(1.0, 1.0, 0.0, 1.0);

        let shader = self.line.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

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

        Program::unbind(&self.gl);

        result
    }

    fn draw_sun_light_gizmos(&self, frame: &DataStreamFrame) -> RenderResult {
        static LINE_COLOR: Vec4 = Vec4::new(0.3, 1.0, 0.3, 1.0);

        let shader = self.line.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        program.set_uniform(&shader.color_location, LINE_COLOR);

        // Draw very long line to represent the sunlight's direction
        let mut result = RenderResult::default();
        for sun_light in frame.sun_lights.iter() {
            let direction = sun_light.direction.normalize();

            // Segment is a 1-unit long line along Z-axis
            let model = Mat4::from_rotation_translation(
                Quat::from_rotation_arc(Vec3::Z, direction),
                -direction * 5000.0,
            ) * Mat4::from_scale(Vec3::splat(10000.0));

            program.set_uniform(&shader.model_location, model);
            result += self.segment.draw();
        }
        Program::unbind(&self.gl);

        result
    }
}

impl RenderPass<RenderingEvent> for GizmosPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_gizmos_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut GizmosPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_gizmos_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.billboard = None;
                self.line = None;
                self.light_texture = None;
            }
            RenderingEvent::ViewportResized(size) => {
                self.viewport_size = size;
            }
            RenderingEvent::UpdateShader(name, shader) if name == BILLBOARD_SHADER.into() => {
                self.billboard = Some(BillboardShader::new(shader.clone()).unwrap());

                let shader = self.billboard.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.texture_location, 0);
                program.set_uniform(&shader.size_location, Vec2::new(0.7, 0.7));
                Program::unbind(&self.gl);
            }

            RenderingEvent::UpdateShader(name, shader) if name == LINE_SHADER.into() => {
                self.line = Some(LineShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.line.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                Program::unbind(&self.gl);
            }

            RenderingEvent::SetLightTexture(texture) => {
                self.light_texture = Some(texture);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "GizmosPass"
    }

    fn begin(
        &mut self,
        _backend: &RendererBackend<RenderingEvent>,
        frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.billboard.is_none() {
            return RenderResult::default();
        }
        if self.line.is_none() {
            return RenderResult::default();
        }
        if self.light_texture.is_none() {
            return RenderResult::default();
        }
        if !self.config.get_show_gizmos() {
            return RenderResult::default();
        }

        Framebuffer::blit_to_default(
            &self.gl,
            &self.gbuffer.fbo,
            self.viewport_size,
            BlitFramebufferMask::Depth,
            BlitFramebufferFilter::Nearest,
        );

        unsafe {
            // Enable blending for transparency
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.enable(glow::DEPTH_TEST);
        }

        let mut result = RenderResult::default();
        result += self.draw_point_light_lines(frame);
        result += self.draw_point_light_billboards(frame);
        result += self.draw_sun_light_gizmos(frame);
        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        if !self.config.get_show_gizmos() {
            return RenderResult::default();
        }

        RenderResult::default()
    }
}
