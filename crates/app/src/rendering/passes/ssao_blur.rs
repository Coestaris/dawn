use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::ssao::{SSAOHalfresTarget, SSAOTarget};
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::ssao_blur::SSAOBlurShader;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;

const DEPTH_INDEX: i32 = 0;
const SSAO_RAW_INDEX: i32 = 1;
const ROUGH_OCCLUSION_NORMAL_INDEX: i32 = 2;

pub(crate) struct SSAOBlurPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,
    shader: Option<SSAOBlurShader>,
    gbuffer: Rc<GBuffer>,
    ssao_raw_taget: Rc<SSAOHalfresTarget>,
    ssao_blur_target: Rc<SSAOTarget>,
    quad: Quad2D,
}

impl SSAOBlurPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        ssao_raw_taget: Rc<SSAOHalfresTarget>,
        ssao_blur_target: Rc<SSAOTarget>,
        config: RenderingConfig,
    ) -> Self {
        SSAOBlurPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            gbuffer,
            ssao_raw_taget,
            ssao_blur_target,
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
            RenderingEvent::ViewportResized(size) => {
                self.ssao_blur_target.resize(size);
            }
            RenderingEvent::UpdateShader(_, shader) => {
                let clone = shader.clone();
                self.shader = Some(SSAOBlurShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera,
                    crate::rendering::CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.ssao_raw, SSAO_RAW_INDEX);
                program.set_uniform(&shader.depth, DEPTH_INDEX);
                program.set_uniform(&shader.rough_occlusion_normal, ROUGH_OCCLUSION_NORMAL_INDEX);

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

        Framebuffer::bind(&self.gl, &self.ssao_blur_target.fbo);

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.clear_color(1.0, 1.0, 1.0, 1.0);
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        #[cfg(feature = "devtools")]
        {
            program.set_uniform(&shader.radius, self.config.get_ssao_blur_radius() as f32);
            program.set_uniform(
                &shader.ssao_enabled,
                self.config.get_is_ssao_enabled() as i32,
            );
            program.set_uniform(
                &shader.sigma_spatial,
                self.config.get_ssao_blur_sigma_spatial(),
            );
            program.set_uniform(&shader.sigma_depth, self.config.get_ssao_blur_sigma_depth());
            program.set_uniform(
                &shader.sigma_normal,
                self.config.get_ssao_blur_sigma_normal(),
            );
        }

        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.ssao_raw_taget.texture.texture,
            SSAO_RAW_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.depth.texture,
            DEPTH_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.rough_occlusion_normal.texture,
            ROUGH_OCCLUSION_NORMAL_INDEX as u32,
        );

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, DEPTH_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, SSAO_RAW_INDEX as u32);
        Texture::unbind(
            &self.gl,
            TextureBind::Texture2D,
            ROUGH_OCCLUSION_NORMAL_INDEX as u32,
        );
        RenderResult::default()
    }
}
