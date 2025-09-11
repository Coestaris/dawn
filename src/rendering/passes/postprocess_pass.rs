use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::obuffer::OBuffer;
use crate::rendering::primitive::quad::Quad;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glow::HasContext;
use std::rc::Rc;

struct ShaderContainer {
    shader: TypedAsset<Program>,

    fxaa_enabled: UniformLocation,
    texture_location: UniformLocation,
    ubo_camera_location: UniformLocation,
}

pub(crate) struct PostProcessPass {
    gl: &'static glow::Context,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ShaderContainer>,
    quad: Quad,
    obuffer: Rc<OBuffer>,
}

impl PostProcessPass {
    pub fn new(
        gl: &'static glow::Context,
        id: RenderPassTargetId,
        obuffer: Rc<OBuffer>,
        config: RenderingConfig,
    ) -> Self {
        PostProcessPass {
            gl,
            id,
            config,
            shader: None,
            quad: Quad::new(gl),
            obuffer,
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
            RenderingEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    fxaa_enabled: shader.cast().get_uniform_location("fxaa_enabled").unwrap(),
                    texture_location: shader.cast().get_uniform_location("in_texture").unwrap(),
                    ubo_camera_location: shader
                        .cast()
                        .get_uniform_block_location("ubo_camera")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(self.gl, &program);
                    program.set_uniform(shader.texture_location, 0);
                    program.set_uniform_block_binding(
                        shader.ubo_camera_location,
                        CAMERA_UBO_BINDING as u32,
                    );
                    Program::unbind(self.gl);
                }
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
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            self.gl.clear(glow::COLOR_BUFFER_BIT);
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        Program::bind(self.gl, program);
        program.set_uniform(
            shader.fxaa_enabled,
            self.config.get_is_fxaa_enabled() as i32,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.obuffer.texture.texture,
            0,
        );

        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(self.gl);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
        RenderResult::default()
    }
}
