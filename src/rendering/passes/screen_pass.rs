use crate::rendering::event::RenderingEvent;
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::primitive::quad::Quad;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::bindings::TEXTURE_2D;
use dawn_graphics::gl::raii::shader_program::{ShaderProgram, UniformLocation};
use dawn_graphics::gl::raii::texture::Texture;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::RendererBackend;
use std::rc::Rc;

struct ShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    color_texture_location: UniformLocation,
}

pub(crate) struct ScreenPass {
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    quad: Quad,
    gbuffer: Rc<GBuffer>,
}

impl ScreenPass {
    pub fn new(id: RenderPassTargetId, gbuffer: Rc<GBuffer>) -> Self {
        ScreenPass {
            id,
            shader: None,
            quad: Quad::new(),
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
                    color_texture_location: shader
                        .cast()
                        .get_uniform_location("color_texture")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    ShaderProgram::bind(&program);
                    program.set_uniform(shader.color_texture_location, 0);
                    ShaderProgram::unbind();
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
    fn begin(&mut self, _: &RendererBackend<RenderingEvent>) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        unsafe {
            bindings::Disable(bindings::DEPTH_TEST);
            bindings::ClearColor(0.1, 0.1, 0.1, 1.0);
            bindings::Clear(bindings::COLOR_BUFFER_BIT);
        }

        let shader = self.shader.as_ref().unwrap();
        ShaderProgram::bind(&shader.shader.cast());
        Texture::bind(TEXTURE_2D, &self.gbuffer.normal_texture.texture, 0);
        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        ShaderProgram::unbind();
        Texture::unbind(TEXTURE_2D, 0);
        RenderResult::default()
    }
}
