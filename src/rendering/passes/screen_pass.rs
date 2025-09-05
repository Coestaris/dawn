use crate::rendering::event::RenderingEvent;
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::primitive::quad::Quad;
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
    shader: TypedAsset<Program<'static>>,
    color_texture_location: UniformLocation,
}

pub(crate) struct ScreenPass<'g> {
    gl: &'g glow::Context,
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    quad: Quad<'g>,
    gbuffer: Rc<GBuffer<'g>>,
}

impl<'g> ScreenPass<'g> {
    pub fn new(gl: &'g glow::Context, id: RenderPassTargetId, gbuffer: Rc<GBuffer<'g>>) -> Self {
        ScreenPass {
            gl,
            id,
            shader: None,
            quad: Quad::new(gl),
            gbuffer,
        }
    }
}

impl<'g> RenderPass<RenderingEvent> for ScreenPass<'g> {
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
                    color_texture_location: shader
                        .cast()
                        .get_uniform_location("color_texture")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(self.gl, &program);
                    program.set_uniform(shader.color_texture_location, 0);
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
        Program::bind(self.gl, &shader.shader.cast());
        Texture::bind(
            self.gl,
            TextureBind::Texture2D,
            &self.gbuffer.albedo_metalic.texture,
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
