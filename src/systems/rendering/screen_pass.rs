use crate::systems::rendering::CustomPassEvent;
use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::bindings::TEXTURE_2D;
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::shader_program::{ShaderProgram, UniformLocation};
use dawn_graphics::gl::raii::texture::Texture;
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::RendererBackend;

struct Quad {
    vao: VertexArray,
    vbo: ArrayBuffer,
    ebo: ElementArrayBuffer,
}

impl Quad {
    fn new() -> Self {
        let vertex: [f32; 16] = [
            // positions   // tex coords
            -1.0, 1.0, 0.0, 1.0, // top left
            -1.0, -1.0, 0.0, 0.0, // bottom left
            1.0, -1.0, 1.0, 0.0, // bottom right
            1.0, 1.0, 1.0, 1.0, // top right
        ];
        let indices_edges: [u16; 6] = [
            0, 1, 2, // first triangle
            0, 2, 3, // second triangle
        ];

        let vao = VertexArray::new(IRTopology::Triangles, IRIndexType::U16).unwrap();
        let mut vbo = ArrayBuffer::new().unwrap();
        let mut ebo = ElementArrayBuffer::new().unwrap();

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        vbo_binding.feed(&vertex, ArrayBufferUsage::StaticDraw);
        ebo_binding.feed(&indices_edges, ElementArrayBufferUsage::StaticDraw);

        vao_binding.setup_attribute(
            0,
            &IRLayout {
                field: IRLayoutField::Position,
                sample_type: IRLayoutSampleType::Float,
                samples: 2,
                stride_bytes: 16,
                offset_bytes: 0,
            },
        );
        vao_binding.setup_attribute(
            1,
            &IRLayout {
                field: IRLayoutField::TexCoord,
                sample_type: IRLayoutSampleType::Float,
                samples: 2,
                stride_bytes: 16,
                offset_bytes: 8,
            },
        );

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        Quad { vao, vbo, ebo }
    }

    fn draw(&self) -> RenderResult {
        let binding = self.vao.bind();
        binding.draw_elements(6, 0)
    }
}

struct ShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    color_texture_location: UniformLocation,
    depth_texture_location: UniformLocation,
}

pub(crate) struct ScreenPass {
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    quad: Quad,

    color_texture: Texture,
    depth_texture: Texture,
}

impl ScreenPass {
    pub fn new(id: RenderPassTargetId, color_texture: Texture, depth_texture: Texture) -> Self {
        ScreenPass {
            id,
            shader: None,
            quad: Quad::new(),
            color_texture,
            depth_texture,
        }
    }
}

impl RenderPass<CustomPassEvent> for ScreenPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_screen_pass(ptr: *mut u8, event: CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut ScreenPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_screen_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: CustomPassEvent) {
        match event {
            CustomPassEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    depth_texture_location: shader
                        .cast()
                        .get_uniform_location("depth_texture")
                        .unwrap(),
                    color_texture_location: shader
                        .cast()
                        .get_uniform_location("color_texture")
                        .unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    ShaderProgram::bind(&program);
                    program.set_uniform(shader.color_texture_location, 0);
                    program.set_uniform(shader.depth_texture_location, 1);
                    ShaderProgram::unbind();
                }
            }
            CustomPassEvent::UpdateWindowSize(size) => {
                Texture::bind(bindings::TEXTURE_2D, &self.color_texture, 0);
                self.color_texture
                    .texture_image_2d(
                        0,
                        size.x as usize,
                        size.y as usize,
                        false,
                        IRPixelFormat::R8G8B8A8,
                        None,
                    )
                    .unwrap();
                self.color_texture.generate_mipmap();
                Texture::unbind(bindings::TEXTURE_2D, 0);

                Texture::bind(bindings::TEXTURE_2D, &self.depth_texture, 0);
                self.depth_texture
                    .texture_image_2d(
                        0,
                        size.x as usize,
                        size.y as usize,
                        false,
                        IRPixelFormat::DEPTH32F,
                        None,
                    )
                    .unwrap();
                self.depth_texture.generate_mipmap();
                Texture::unbind(bindings::TEXTURE_2D, 0);
            }
            CustomPassEvent::DropAllAssets => {
                self.shader = None;
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "ScreenPass"
    }

    #[inline(always)]
    fn begin(&mut self, _: &RendererBackend<CustomPassEvent>) -> RenderResult {
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
        Texture::bind(TEXTURE_2D, &self.color_texture, 0);
        Texture::bind(TEXTURE_2D, &self.depth_texture, 1);
        self.quad.draw()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<CustomPassEvent>) -> RenderResult {
        ShaderProgram::unbind();
        Texture::unbind(TEXTURE_2D, 0);
        Texture::unbind(TEXTURE_2D, 1);
        RenderResult::default()
    }
}
