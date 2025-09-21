use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::primitive::quad::Quad2D;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::sync::Arc;
use crate::rendering::shaders::ssao_blur::SSAOBlurShader;
use crate::rendering::ubo::CAMERA_UBO_BINDING;

pub(crate) struct SSAOBlurPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,
    shader: Option<SSAOBlurShader>,
    quad: Quad2D,
}

impl SSAOBlurPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        config: RenderingConfig,
    ) -> Self {
        SSAOBlurPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            quad: Quad2D::new(gl),
        }
    }
}

impl RenderPass<RenderingEvent> for SSAOBlurPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut SSAOBlurPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                let clone = shader.clone();
                self.shader = Some(SSAOBlurShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
 
                Program::unbind(&self.gl);
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "SSAOBlur"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        RenderResult::default()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, 0);
        RenderResult::default()
    }
}
