use crate::components::imui::UICommand;
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
use glam::{Mat4, UVec2, Vec2, Vec3, Vec4};
use log::warn;
use triple_buffer::Output;

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
    stream: Output<Vec<UICommand>>,
}

impl UIPass {
    pub fn new(id: RenderPassTargetId, stream: Output<Vec<UICommand>>) -> Self {
        UIPass {
            id,
            shader: None,
            projection: Mat4::IDENTITY,
            stream,
        }
    }

    fn calculate_projection(&mut self, win_size: UVec2) {
        self.projection =
            Mat4::orthographic_rh_gl(0.0, win_size.x as f32, win_size.y as f32, 0.0, -1.0, 1.0);
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

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "UIPass"
    }

    fn begin(&mut self, _backend: &RendererBackend<CustomPassEvent>) -> RenderResult {
        // return RenderResult::default();

        if let None = self.shader {
            return RenderResult::default();
        }

        // Disable depth testing for UI
        unsafe {
            bindings::Disable(bindings::DEPTH_TEST);
            bindings::Enable(bindings::BLEND);
            bindings::BlendFunc(bindings::SRC_ALPHA, bindings::ONE_MINUS_SRC_ALPHA);
            bindings::Disable(bindings::CULL_FACE);
        }

        let commands = self.stream.read();
        let mut result = RenderResult::default();

        let mut style = None;
        let mut color = Vec4::new(1.0, 1.0, 1.0, 1.0);

        for command in commands {
            match command {
                UICommand::ApplyStyle(new_style) => {
                    style = Some(new_style);
                }
                UICommand::ChangeColor(new_color) => {
                    color = *new_color;
                }
                UICommand::Box(_pos, _size) => {
                    unimplemented!()
                }

                UICommand::StaticText(pos, str) => {
                    if style.is_none() {
                        warn!("UI Command StaticText received before ApplyStyle, skipping");
                        continue;
                    }
                    let style = style.as_ref().unwrap();
                    let render = StringRender::new(
                        &style.font,
                        self.shader.as_ref().unwrap(),
                        color,
                        style.scale,
                    );
                    result += render.render_text(str, *pos);
                }

                UICommand::Text(pos, str) => {
                    if style.is_none() {
                        warn!("UI Command StaticText received before ApplyStyle, skipping");
                        continue;
                    }
                    let style = style.as_ref().unwrap();
                    let render = StringRender::new(
                        &style.font,
                        self.shader.as_ref().unwrap(),
                        color,
                        style.scale,
                    );
                    result += render.render_text(&str, *pos);
                }
            }
        }

        RenderResult::ok(0, 0)
    }

    fn end(&mut self, _backend: &mut RendererBackend<CustomPassEvent>) -> RenderResult {
        ShaderProgram::unbind();
        Texture::unbind(bindings::TEXTURE_2D, 0);

        // Re-enable depth testing after UI pass
        unsafe {
            bindings::Enable(bindings::CULL_FACE);
            bindings::Enable(bindings::DEPTH_TEST);
            bindings::DepthFunc(bindings::LEQUAL);
            bindings::Disable(bindings::BLEND);
        }

        RenderResult::ok(0, 0)
    }
}

struct StringRender<'a> {
    glyph_shader: &'a GlyphShaderContainer,
    font: &'a Font,
    atlas: &'a Texture,
    scale: f32,
    color: Vec4,
}

impl<'a> StringRender<'a> {
    fn new(
        font_asset: &'a TypedAsset<Font>,
        shader: &'a GlyphShaderContainer,
        color: Vec4,
        scale: f32,
    ) -> Self {
        let font = font_asset.cast();
        let atlas = font.atlas.cast::<Texture>();

        let program = shader.shader.cast();
        ShaderProgram::bind(program);
        // Assume projection and atlas location is already set
        program.set_uniform(shader.color_location, color);
        Texture::bind(bindings::TEXTURE_2D, atlas, 0);

        StringRender {
            glyph_shader: shader,
            font,
            atlas,
            scale,
            color,
        }
    }

    pub fn render_text(&self, str: &str, start_position: Vec2) -> RenderResult {
        let shader = self.glyph_shader.shader.cast();

        let mut position = start_position;
        self.font.render_string(str, |char, glyph| {
            match char {
                ' ' => {
                    position += Vec2::new(self.font.space_advance * self.scale, 0.0); // Simple space handling
                    return (true, RenderResult::default());
                }

                '\n' => {
                    position.x = start_position.x;
                    position.y += self.font.y_advance * self.scale;
                    return (true, RenderResult::default());
                }

                _ => {}
            }
            let glyph = glyph.unwrap();

            // Something
            let model = Mat4::from_translation(Vec3::new(
                position.x + glyph.x_offset * self.scale,
                position.y + glyph.y_offset * self.scale,
                0.0,
            ));
            let model = model * Mat4::from_scale(Vec3::splat(self.scale));

            shader.set_uniform(self.glyph_shader.model_location, model);
            position += Vec2::new(glyph.x_advance * self.scale, 0.0);
            (false, RenderResult::default())
        })
    }
}

impl Drop for StringRender<'_> {
    fn drop(&mut self) {
        ShaderProgram::unbind();
        Texture::unbind(bindings::TEXTURE_2D, 0);
    }
}
