use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::fbo::ssao::SSAOTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::ssao_raw::SSAORawShader;
use crate::rendering::textures::noise::{white_noise_tangent_space_f32};
use crate::rendering::ubo::ssao_raw::{SSAORawKernelUBO, SSAORawParametersUBO};
use crate::rendering::ubo::{
    CAMERA_UBO_BINDING, SSAO_RAW_KERNEL_UBO_BINDING, SSAO_RAW_PARAMS_UBO_BINDING,
};
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

const POSITION_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const NOISE_INDEX: i32 = 2;

pub(crate) struct SSAORawPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    shader: Option<SSAORawShader>,
    target: Rc<SSAOTarget>,

    config: RenderingConfig,
    gbuffer: Rc<GBuffer>,

    noise_texture: Texture,
    quad: Quad2D,

    params_ubo: SSAORawParametersUBO,
    kernel_ubo: SSAORawKernelUBO,
}

impl SSAORawPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        target: Rc<SSAOTarget>,
        config: RenderingConfig,
    ) -> Self {
        SSAORawPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            noise_texture: white_noise_tangent_space_f32(gl.clone(), 4, 4),
            quad: Quad2D::new(gl.clone()),
            params_ubo: SSAORawParametersUBO::new(gl.clone(), SSAO_RAW_PARAMS_UBO_BINDING),
            kernel_ubo: SSAORawKernelUBO::new(gl.clone(), SSAO_RAW_KERNEL_UBO_BINDING),
            gbuffer,
            target,
        }
    }
}

impl RenderPass<RenderingEvent> for SSAORawPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut SSAORawPass) };
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
            }
            RenderingEvent::UpdateShader(_, shader) => {
                let clone = shader.clone();
                self.shader = Some(SSAORawShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_ssao_raw_kernel_location,
                    SSAO_RAW_KERNEL_UBO_BINDING as u32,
                );
                program.set_uniform_block_binding(
                    shader.ubo_ssao_raw_params_location,
                    SSAO_RAW_PARAMS_UBO_BINDING as u32,
                );
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.position_location, POSITION_INDEX);
                // program.set_uniform(&shader.depth_location, DEPTH_INDEX);
                program.set_uniform(&shader.normal_location, NORMAL_INDEX);
                program.set_uniform(&shader.noise_location, NOISE_INDEX);
                Program::unbind(&self.gl);
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "SSAORaw"
    }

    fn begin(
        &mut self,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        // Drawing offscreen to SSAO target
        Framebuffer::bind(&self.gl, &self.target.fbo);

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
            self.gl.clear_color(1.0, 1.0, 1.0, 1.0);
        }

        // Update params UBO
        self.params_ubo
            .set_kernel_size(self.config.get_ssao_kernel_size());
        self.params_ubo.set_radius(self.config.get_ssao_radius());
        self.params_ubo.set_bias(self.config.get_ssao_bias());
        self.params_ubo
            .set_intensity(self.config.get_ssao_intensity());
        self.params_ubo.set_power(self.config.get_ssao_power());
        if self.params_ubo.upload() {
            self.kernel_ubo
                .set_samples(self.config.get_ssao_kernel_size() as usize);
            self.kernel_ubo.upload();
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.position.texture,
            POSITION_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.normal.texture,
            NORMAL_INDEX as u32,
        );
        Texture::bind(
            &self.gl,
            TextureBind::Texture2D,
            &self.noise_texture,
            NOISE_INDEX as u32,
        );

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, POSITION_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, NORMAL_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, NOISE_INDEX as u32);
        RenderResult::default()
    }
}
