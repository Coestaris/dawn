use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::halfres::HalfresBuffer;
use crate::rendering::fbo::ssao::SSAOHalfresTarget;
use crate::rendering::primitive::quad::Quad2D;
use crate::rendering::shaders::ssao_blur::SSAOBlurShader;
use crate::rendering::ubo::ssao_blur::SSAOBlurKernelUBO;
use crate::rendering::ubo::SSAO_BLUR_KERNEL_UBO_BINDING;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::Texture2D;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{UVec2, Vec2};
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

const HALFRES_SSAO_RAW_INDEX: i32 = 0;
const HALFRES_NORMAL_INDEX: i32 = 1;
const HALFRES_DEPTH_INDEX: i32 = 2;

struct Devtools {
    ubo: SSAOBlurKernelUBO,
    prev_taps_count: u32,
    prev_sigma_spatial: f32,
}

enum RenderMode {
    Horizontal,
    Vertical,
}

impl Devtools {
    fn new(gl: Arc<glow::Context>) -> Self {
        Devtools {
            ubo: SSAOBlurKernelUBO::new(gl, SSAO_BLUR_KERNEL_UBO_BINDING),
            prev_taps_count: 0,
            prev_sigma_spatial: 0.0,
        }
    }
}

pub(crate) struct SSAOBlurPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    shader: Option<SSAOBlurShader>,
    target: Rc<SSAOHalfresTarget>,

    viewport: UVec2,
    config: RenderingConfig,
    halfres_buffer: Rc<HalfresBuffer>,
    halfres_ssao_raw: Rc<SSAOHalfresTarget>,

    quad: Quad2D,

    #[cfg(feature = "devtools")]
    devtools: Devtools,
}

impl SSAOBlurPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        halfres_buffer: Rc<HalfresBuffer>,
        halfres_ssao_raw: Rc<SSAOHalfresTarget>,
        target: Rc<SSAOHalfresTarget>,
        config: RenderingConfig,
    ) -> Self {
        SSAOBlurPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            halfres_buffer,
            target,
            viewport: Default::default(),
            quad: Quad2D::new(gl.clone()),
            halfres_ssao_raw,

            #[cfg(feature = "devtools")]
            devtools: Devtools::new(gl.clone()),
        }
    }

    fn setup_render(&mut self) {
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

        #[cfg(feature = "devtools")]
        {
            program.set_uniform(
                &shader.devtools.tap_count,
                self.config.get_ssao_blur_taps_count() as i32,
            );
            program.set_uniform(
                &shader.devtools.sigma_depth,
                self.config.get_ssao_blur_sigma_depth(),
            );
            program.set_uniform(
                &shader.devtools.ssao_enabled,
                self.config.get_is_ssao_enabled() as i32,
            );

            if self.config.get_ssao_blur_taps_count() != self.devtools.prev_taps_count
                || (self.config.get_ssao_blur_sigma_spatial() - self.devtools.prev_sigma_spatial)
                    .abs()
                    > f32::EPSILON
            {
                self.devtools.prev_taps_count = self.config.get_ssao_blur_taps_count();
                self.devtools.prev_sigma_spatial = self.config.get_ssao_blur_sigma_spatial();
                self.devtools.ubo.set_samples(
                    self.config.get_ssao_blur_tap_weight(),
                    self.config.get_ssao_blur_tap_offset(),
                );
                self.devtools.ubo.upload();
            }
        }

        self.halfres_buffer.normal.bind2d(HALFRES_NORMAL_INDEX);
        self.halfres_buffer.depth.bind2d(HALFRES_DEPTH_INDEX);
    }

    fn render(
        &self,
        mode: RenderMode,
        raw: &SSAOHalfresTarget,
        target: &SSAOHalfresTarget,
    ) -> RenderResult {
        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();

        let stride = match mode {
            RenderMode::Horizontal => Vec2::new(1.0 / (self.viewport.x as f32 / 2.0), 0.0),
            RenderMode::Vertical => Vec2::new(0.0, 1.0 / (self.viewport.y as f32 / 2.0)),
        };

        // Bind target framebuffer
        Framebuffer::bind(&self.gl, &target.fbo);
        // Bind inputs
        program.set_uniform(&shader.stride, stride);
        raw.texture.bind2d(HALFRES_SSAO_RAW_INDEX);

        let result = self.quad.draw(&self.gl);

        // Unbind
        Framebuffer::unbind(&self.gl);
        Texture2D::unbind(&self.gl, HALFRES_SSAO_RAW_INDEX as u32);

        result
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
                self.target.resize(size);
                self.viewport = size;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(SSAOBlurShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera,
                    crate::rendering::CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.halfres_ssao_raw, HALFRES_SSAO_RAW_INDEX);
                program.set_uniform(&shader.halfres_normal, HALFRES_NORMAL_INDEX);
                program.set_uniform(&shader.halfres_depth, HALFRES_DEPTH_INDEX);

                #[cfg(feature = "devtools")]
                program.set_uniform_block_binding(
                    shader.devtools.ubo_ssao_blur_taps,
                    SSAO_BLUR_KERNEL_UBO_BINDING as u32,
                );

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
        _: &Window,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        self.setup_render();

        let mut result = RenderResult::default();

        result += self.render(RenderMode::Horizontal, &self.halfres_ssao_raw, &self.target);
        result += self.render(RenderMode::Vertical, &self.target, &self.halfres_ssao_raw);

        result
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
        Texture2D::unbind(&self.gl, HALFRES_SSAO_RAW_INDEX as u32);
        Texture2D::unbind(&self.gl, HALFRES_NORMAL_INDEX as u32);
        RenderResult::default()
    }
}
