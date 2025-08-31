use crate::systems::rendering::CustomPassEvent;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::font::Font;
use dawn_graphics::gl::raii::shader_program::{ShaderProgram, UniformLocation};
use dawn_graphics::gl::raii::texture::Texture;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::RendererBackend;
use glam::{Mat4, UVec2, Vec2};

struct GlyphShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    proj_location: UniformLocation,
    color_location: UniformLocation,
    atlas_location: UniformLocation,
}

pub(crate) struct UIPass {
    id: RenderPassTargetId,
    shader: Option<GlyphShaderContainer>,
    projection: Mat4,
    font: Option<TypedAsset<Font>>,
}

impl UIPass {
    pub fn new(id: RenderPassTargetId) -> Self {
        UIPass {
            id,
            shader: None,
            projection: Mat4::IDENTITY,
            font: None,
        }
    }

    fn calculate_projection(&mut self, win_size: UVec2) {
        self.projection =
            Mat4::orthographic_rh_gl(0.0, win_size.x as f32, 0.0, win_size.y as f32, -1.0, 1.0);
    }

    fn set_projection(&mut self) {
        if let Some(shader) = self.shader.as_mut() {
            let program = shader.shader.cast();
            ShaderProgram::bind(program);
            program.set_uniform(shader.proj_location, self.projection);
            program.set_uniform(shader.atlas_location, 0);
            ShaderProgram::unbind();
        }
    }
}

impl RenderPass<CustomPassEvent> for UIPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_ui_pass(ptr: *mut u8, event: CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut UIPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_ui_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: CustomPassEvent) {
        match event {
            CustomPassEvent::DropAllAssets => {
                self.shader = None;
                self.font = None;
            }
            CustomPassEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                let casted = shader.cast();
                self.shader = Some(GlyphShaderContainer {
                    shader: clone,
                    model_location: casted.get_uniform_location("model").unwrap(),
                    proj_location: casted.get_uniform_location("projection").unwrap(),
                    color_location: casted.get_uniform_location("color").unwrap_or(0),
                    atlas_location: casted.get_uniform_location("atlas").unwrap(),
                });
                self.set_projection();
            }
            CustomPassEvent::UpdateWindowSize(size) => {
                self.calculate_projection(size);
                self.set_projection();
            }
            CustomPassEvent::UpdateFont(font) => {
                self.font = Some(font.clone());
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "UIPass"
    }

    fn begin(&mut self, _backend: &RendererBackend<CustomPassEvent>) -> RenderResult {
        if let None = self.shader {
            return RenderResult::default();
        }
        if let None = self.font {
            return RenderResult::default();
        }
        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();

        ShaderProgram::bind(&program);
        // program.set_uniform(shader.color_location, Vec2::new(1.0, 1.0));

        let font = self.font.as_ref().unwrap().cast();
        let atlas = font.atlas.cast::<Texture>();
        Texture::bind(bindings::TEXTURE_2D, atlas, 0);

        let string = "123456";
        let mut text_position = Vec2::new(25.0, 25.0);
        font.render_string(string, |glyph| {
            // Something
            // let model = Mat4::from_translation(Vec3::new(
            //     text_position.x + glyph.x_offset,
            //     text_position.y - glyph.y_offset,
            //     0.0,
            // ));
            let model = Mat4::IDENTITY;
            program.set_uniform(shader.model_location, model);
            text_position += Vec2::new(glyph.x_advance, 0.0);
            (false, RenderResult::default())
        });

        ShaderProgram::bind(program);
        RenderResult::ok(0, 0)
    }

    fn end(&mut self, _backend: &mut RendererBackend<CustomPassEvent>) -> RenderResult {
        ShaderProgram::unbind();
        Texture::unbind(bindings::TEXTURE_2D, 0);
        RenderResult::ok(0, 0)
    }
}
