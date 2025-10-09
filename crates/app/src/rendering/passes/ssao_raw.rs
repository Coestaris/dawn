use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::halfres::HalfresBuffer;
use crate::rendering::fbo::ssao::SSAOHalfresTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::ssao_raw::SSAORawShader;
use crate::rendering::ubo::ssao_raw::SSAORawKernelUBO;
use crate::rendering::ubo::{CAMERA_UBO_BINDING, SSAO_RAW_KERNEL_UBO_BINDING};
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::UVec2;
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

const HALFRES_DEPTH_INDEX: i32 = 0;
const HALFRES_NORMAL_INDEX: i32 = 1;

pub(crate) struct SSAORawPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    shader: Option<SSAORawShader>,
    target: Rc<SSAOHalfresTarget>,

    viewport: UVec2,
    config: RenderingConfig,
    halfres_buffer: Rc<HalfresBuffer>,

    quad: Quad2D,

    #[cfg(feature = "devtools")]
    kernel_ubo: SSAORawKernelUBO,
    #[cfg(feature = "devtools")]
    prev_kernel_size: usize,
}

impl SSAORawPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        halfres_buffer: Rc<HalfresBuffer>,
        target: Rc<SSAOHalfresTarget>,
        config: RenderingConfig,
    ) -> Self {
        SSAORawPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            quad: Quad2D::new(gl.clone()),
            halfres_buffer,
            target,
            viewport: Default::default(),
            #[cfg(feature = "devtools")]
            prev_kernel_size: 0,
            #[cfg(feature = "devtools")]
            kernel_ubo: SSAORawKernelUBO::new(gl.clone(), SSAO_RAW_KERNEL_UBO_BINDING),
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
                self.viewport = size;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(SSAORawShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);

                program.set_uniform_block_binding(shader.ubo_camera, CAMERA_UBO_BINDING as u32);
                program.set_uniform(&shader.halfres_depth, HALFRES_DEPTH_INDEX);
                program.set_uniform(&shader.halfres_normal, HALFRES_NORMAL_INDEX);

                #[cfg(feature = "devtools")]
                program.set_uniform_block_binding(
                    shader.devtools.ubo_ssao_raw_kernel,
                    SSAO_RAW_KERNEL_UBO_BINDING as u32,
                );

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
        _: &Window,
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
            // self.gl.clear(glow::COLOR_BUFFER_BIT);
            // self.gl.clear_color(1.0, 1.0, 1.0, 1.0);
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

        #[cfg(feature = "devtools")]
        {
            program.set_uniform(
                &shader.devtools.kernel_size,
                self.config.get_ssao_raw_kernel_size() as i32,
            );
            program.set_uniform(&shader.devtools.radius, self.config.get_ssao_raw_radius());
            program.set_uniform(&shader.devtools.bias, self.config.get_ssao_raw_bias());
            program.set_uniform(&shader.devtools.intensity, self.config.get_ssao_raw_intensity());
            program.set_uniform(&shader.devtools.power, self.config.get_ssao_raw_power());
            program.set_uniform(
                &shader.devtools.ssao_enabled,
                self.config.get_is_ssao_enabled() as i32,
            );
            if self.prev_kernel_size != self.config.get_ssao_raw_kernel_size() as usize {
                self.prev_kernel_size = self.config.get_ssao_raw_kernel_size() as usize;
                self.kernel_ubo.set_samples(self.config.get_ssao_raw_kernel());
                self.kernel_ubo.upload();
            }
        }

        self.halfres_buffer.depth.bind2d(HALFRES_DEPTH_INDEX);
        self.halfres_buffer.normal.bind2d(HALFRES_NORMAL_INDEX);

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        unsafe {
            // Restore viewport to full resolution
            self.gl
                .viewport(0, 0, self.viewport.x as i32, self.viewport.y as i32);
        }

        Program::unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, HALFRES_DEPTH_INDEX as u32);
        Texture::unbind(
            &self.gl,
            TextureBind::Texture2D,
            HALFRES_NORMAL_INDEX as u32,
        );
        RenderResult::default()
    }
}
