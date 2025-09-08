use crate::rendering::event::RenderingEvent;
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::primitive::quad::Quad;
use crate::rendering::ui::{OutputMode, RenderingConfig};
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

    debug_mode: UniformLocation,

    albedo_metallic_texture: UniformLocation,
    normal_texture: UniformLocation,
    pbr_texture: UniformLocation,
}

pub(crate) struct ScreenPass {
    gl: &'static glow::Context,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ShaderContainer>,
    quad: Quad,
    gbuffer: Rc<GBuffer>,
}

impl ScreenPass {
    pub fn new(
        gl: &'static glow::Context,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        config: RenderingConfig,
    ) -> Self {
        ScreenPass {
            gl,
            id,
            config,
            shader: None,
            quad: Quad::new(gl),
            gbuffer,
        }
    }
}

impl RenderPass<RenderingEvent> for ScreenPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_screen_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut ScreenPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_screen_pass, self.id, self)]
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
                    debug_mode: shader.cast().get_uniform_location("in_debug_mode").unwrap(),
                    albedo_metallic_texture: shader
                        .cast()
                        .get_uniform_location("in_albedo_metallic_texture")
                        .unwrap(),
                    normal_texture: shader
                        .cast()
                        .get_uniform_location("in_normal_texture")
                        .unwrap(),
                    pbr_texture: shader
                        .cast()
                        .get_uniform_location("in_pbr_texture")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(self.gl, &program);
                    program.set_uniform(shader.albedo_metallic_texture, 0);
                    program.set_uniform(shader.normal_texture, 1);
                    program.set_uniform(shader.pbr_texture, 2);
                    Program::unbind(self.gl);
                }
            }
            RenderingEvent::ViewportResized(size) => self.gbuffer.resize(size),

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "ScreenPass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &RendererBackend<RenderingEvent>,
        frame: &DataStreamFrame,
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
        program.set_uniform(shader.debug_mode, self.config.borrow().output_mode as i32);
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.albedo_metalic.texture,
            0,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.normal.texture,
            1,
        );
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.pbr.texture,
            2,
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
