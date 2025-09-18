use crate::rendering::config::{BoundingBoxMode, RenderingConfig};
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::primitive::cube_lines::CubeLines;
use crate::rendering::shaders::LineShader;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::framebuffer::{
    BlitFramebufferFilter, BlitFramebufferMask, Framebuffer,
};
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{Mat4, UVec2, Vec3, Vec4};
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct BoundingPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    cube: CubeLines,
    shader: Option<LineShader>,
    viewport_size: UVec2,
    gbuffer: Rc<GBuffer>,
    config: RenderingConfig,
}

impl BoundingPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        config: RenderingConfig,
    ) -> Self {
        BoundingPass {
            gl: gl.clone(),
            id,
            shader: None,
            cube: CubeLines::new(gl),
            viewport_size: UVec2::ZERO,
            gbuffer,
            config,
        }
    }
}

impl RenderPass<RenderingEvent> for BoundingPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_bounding_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut BoundingPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_bounding_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }

            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(LineShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                Program::unbind(&self.gl);
            }

            RenderingEvent::ViewportResized(size) => {
                self.viewport_size = size;
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "BoundingPass"
    }

    fn begin(
        &mut self,
        _backend: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        match self.config.get_bounding_box_mode() {
            BoundingBoxMode::Disabled => {
                return RenderResult::default();
            }
            BoundingBoxMode::AABBHonorDepth | BoundingBoxMode::OBBHonorDepth => {
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
            }
            _ => {}
        }

        // Bind shader
        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        RenderResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<RenderingEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        if matches!(
            self.config.get_bounding_box_mode(),
            BoundingBoxMode::Disabled
        ) {
            return RenderResult::default();
        }

        let mesh = renderable.mesh.cast();
        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        let mut result = RenderResult::default();

        static MESH_COLOR: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        static SUBMESH_COLOR: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);

        fn draw_cube(
            pass: &BoundingPass,
            renderable_model: Mat4,
            min: Vec3,
            max: Vec3,
        ) -> RenderResult {
            let shader = pass.shader.as_ref().unwrap();
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

        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if matches!(
            self.config.get_bounding_box_mode(),
            BoundingBoxMode::Disabled
        ) {
            return RenderResult::default();
        }

        // Unbind shader
        Program::unbind(&self.gl);

        RenderResult::default()
    }
}
