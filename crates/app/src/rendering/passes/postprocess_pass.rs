use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::lighting::LightingTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::postprocess::PostprocessShader;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::Texture2D;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

pub(crate) struct PostProcessPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<PostprocessShader>,
    quad: Quad2D,
    lightning_target: Rc<LightingTarget>,
}

impl PostProcessPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        lightning_target: Rc<LightingTarget>,
        config: RenderingConfig,
    ) -> Self {
        PostProcessPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            quad: Quad2D::new(gl),
            lightning_target,
        }
    }
}

impl RenderPass<RenderingEvent> for PostProcessPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut PostProcessPass) };
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
                self.shader = Some(PostprocessShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform(&shader.texture_location, 0);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                Program::unbind(&self.gl);
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "PostProcessPass"
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

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            // self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            // self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, program);
        program.set_uniform(
            &shader.fxaa_enabled,
            self.config.get_is_fxaa_enabled() as i32,
        );
        self.lightning_target.texture.bind2d(0);

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(&self.gl);
        Texture2D::unbind(&self.gl, 0);
        RenderResult::default()
    }
}
