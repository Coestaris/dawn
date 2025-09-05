use crate::rendering::event::RenderingEvent;
use crate::world::ui::{UICommand, UIReader};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::font::Font;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{Mat4, Vec2, Vec3, Vec4};
use glow::HasContext;
use log::warn;

struct ShaderContainer {
    shader: TypedAsset<Program<'static>>,
    model_location: UniformLocation,
    proj_location: UniformLocation,
    color_location: UniformLocation,
    atlas_location: UniformLocation,
}

pub(crate) struct UIPass<'g> {
    gl: &'g glow::Context,
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    projection: Mat4,
    reader: UIReader,
}

impl<'g> UIPass<'g> {
    pub fn new(gl: &'g glow::Context, id: RenderPassTargetId, reader: UIReader) -> Self {
        UIPass {
            gl,
            id,
            shader: None,
            projection: Mat4::IDENTITY,
            reader,
        }
    }

    fn set_projection(&mut self) {
        if let Some(shader) = self.shader.as_mut() {
            let program = shader.shader.cast();
            Program::bind(self.gl, program);
            program.set_uniform(shader.proj_location, self.projection);
            program.set_uniform(shader.atlas_location, 0);
            Program::unbind(self.gl);
        }
    }
}

impl<'g> RenderPass<RenderingEvent> for UIPass<'g> {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_ui_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut UIPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_ui_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;

                // I know this is ugly, but it works.
                // Hoping the renderer will not be created more than once
                let data = self.reader.get_data_mut();
                data.clear();
                self.reader.update();
            }
            RenderingEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                let casted = shader.cast();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    model_location: casted.get_uniform_location("model").unwrap(),
                    proj_location: casted.get_uniform_location("projection").unwrap(),
                    color_location: casted.get_uniform_location("color").unwrap(),
                    atlas_location: casted.get_uniform_location("atlas").unwrap(),
                });
                self.set_projection();
            }
            RenderingEvent::OrthographicProjectionUpdated(proj) => {
                self.projection = proj;
                self.set_projection();
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "UIPass"
    }

    fn begin(
        &mut self,
        _backend: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        // return RenderResult::default();

        if let None = self.shader {
            return RenderResult::default();
        }

        // Disable depth testing for UI
        unsafe {
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            self.gl.disable(glow::CULL_FACE);
        }

        let commands = self.reader.get_data();
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
                        self.gl,
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
                        self.gl,
                        &style.font,
                        self.shader.as_ref().unwrap(),
                        color,
                        style.scale,
                    );
                    result += render.render_text(&str, *pos);
                }
            }
        }

        self.reader.update();
        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        Program::unbind(self.gl);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);

        // Re-enable depth testing after UI pass
        unsafe {
            self.gl.enable(glow::CULL_FACE);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LEQUAL);
            self.gl.depth_func(glow::BLEND);
        }

        RenderResult::ok(0, 0)
    }
}

struct StringRender<'g, 'a> {
    gl: &'g glow::Context,
    glyph_shader: &'a ShaderContainer,
    font: &'a Font<'g>,
    atlas: &'a Texture<'g>,
    scale: f32,
    color: Vec4,
}

impl<'g, 'a> StringRender<'g, 'a> {
    fn new(
        gl: &'g glow::Context,
        font_asset: &'a TypedAsset<Font<'static>>,
        shader: &'a ShaderContainer,
        color: Vec4,
        scale: f32,
    ) -> Self {
        let font = font_asset.cast();
        let atlas = font.atlas.cast::<Texture>();

        let program = shader.shader.cast();
        Program::bind(gl, program);
        // Assume projection and atlas location is already set
        program.set_uniform(shader.color_location, color);
        Texture::bind(gl, TextureBind::Texture2D, atlas, 0);

        StringRender {
            gl,
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

impl<'g> Drop for StringRender<'g, '_> {
    fn drop(&mut self) {
        Program::unbind(self.gl);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
    }
}
