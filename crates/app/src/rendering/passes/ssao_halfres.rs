use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::halfres::HalfresBuffer;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::ssao_halfres::SSAOHalfresShader;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::Texture2D;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::UVec2;
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

const DEPTH_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;

pub(crate) struct SSAOHalfresPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    shader: Option<SSAOHalfresShader>,
    gbuffer: Rc<GBuffer>,
    target: Rc<HalfresBuffer>,
    viewport: UVec2,
    quad: Quad2D,
}

impl SSAOHalfresPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        target: Rc<HalfresBuffer>,
    ) -> Self {
        SSAOHalfresPass {
            gl: gl.clone(),
            id,
            shader: None,
            gbuffer,
            target,
            viewport: UVec2::ZERO,
            quad: Quad2D::new(gl),
        }
    }
}

impl RenderPass<RenderingEvent> for SSAOHalfresPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut SSAOHalfresPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::ViewportResized(size) => {
                self.target.resize(size);
                self.viewport = size;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(SSAOHalfresShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera,
                    crate::rendering::CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.depth, DEPTH_INDEX);
                program.set_uniform(&shader.normal, NORMAL_INDEX);

                Program::unbind(&self.gl);
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "SSAOHalfres"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &Window,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        Framebuffer::bind(&self.gl, &self.target.fbo);

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);

            // Rendering in half resolution
            self.gl.viewport(
                0,
                0,
                (self.viewport.x / 2) as i32,
                (self.viewport.y / 2) as i32,
            );
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        self.gbuffer.depth.bind2d(DEPTH_INDEX);
        self.gbuffer.normal.bind2d(NORMAL_INDEX);

        self.quad.draw(&self.gl)
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        Texture2D::unbind(&self.gl, DEPTH_INDEX as u32);
        Texture2D::unbind(&self.gl, NORMAL_INDEX as u32);
        RenderResult::default()
    }
}
