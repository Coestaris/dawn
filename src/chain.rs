use dawn_assets::TypedAsset;
use dawn_graphics::gl::entities::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::entities::element_array_buffer::{
    ElementArrayBuffer, ElementArrayBufferUsage,
};
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::gl::entities::shader_program::UniformLocation;
use dawn_graphics::gl::entities::texture::Texture;
use dawn_graphics::gl::entities::vertex_array::{
    DrawElementsMode, VertexArray, VertexAttribute, VertexAttributeFormat,
};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::PassExecuteResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::RendererBackend;
use glam::{Mat4, Vec3};
use log::info;

pub struct Mesh {
    vao: VertexArray,
    vbo: ArrayBuffer,
    ebo: ElementArrayBuffer,
    count: usize,
}

pub fn create_quad() -> Mesh {
    let vertices: [f32; 32] = [
        // positions          // texture coords
        0.5, 0.5, 0.0, 0.5, 0.5, 0.0, 1.0, 1.0, // top right
        0.5, -0.5, 0.0, 0.5, -0.5, 0.0, 1.0, 0.0, // bottom right
        -0.5, -0.5, 0.0, -0.5, -0.5, 0.0, 0.0, 0.0, // bottom left
        -0.5, 0.5, 0.0, -0.5, 0.5, 0.0, 0.0, 1.0, // top left
    ];
    let indices: [u32; 6] = [
        // note that we start from 0!
        0, 1, 3, // first Triangle
        1, 2, 3, // second Triangle
    ];

    let vao = VertexArray::new().unwrap();
    let mut vbo = ArrayBuffer::new().unwrap();
    let mut ebo = ElementArrayBuffer::new().unwrap();

    let vao_binding = vao.bind();
    let vbo_binding = vbo.bind();
    let ebo_binding = ebo.bind();

    vbo_binding
        .feed(&vertices, ArrayBufferUsage::StaticDraw)
        .unwrap();
    ebo_binding
        .feed(&indices, ElementArrayBufferUsage::StaticDraw)
        .unwrap();

    vao_binding
        .setup_attribute(VertexAttribute {
            id: 0,
            sample_size: 3,
            format: VertexAttributeFormat::Float32,
            stride_bytes: 8 * 4,
            offset_bytes: 0,
        })
        .unwrap();
    vao_binding
        .setup_attribute(VertexAttribute {
            id: 1,
            sample_size: 3,
            format: VertexAttributeFormat::Float32,
            stride_bytes: 8 * 4,
            offset_bytes: 3 * 4,
        })
        .unwrap();
    vao_binding
        .setup_attribute(VertexAttribute {
            id: 2,
            sample_size: 2,
            format: VertexAttributeFormat::Float32,
            stride_bytes: 8 * 4,
            offset_bytes: 6 * 4,
        })
        .unwrap();

    drop(vbo_binding);
    drop(ebo_binding);
    drop(vao_binding);

    Mesh {
        vao,
        vbo,
        ebo,
        count: 6,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    UpdateShader(TypedAsset<ShaderProgram>),
    ChangeColor(Vec3),
    UpdateTexture(TypedAsset<Texture>),
    UpdateWindowSize(usize, usize),
}

struct TriangleShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
    texture_uniform: UniformLocation,
}

struct TextureContainer {
    texture: TypedAsset<Texture>,
}

pub(crate) struct GeometryPass {
    id: RenderPassTargetId,
    shader: Option<TriangleShaderContainer>,
    texture: Option<TextureContainer>,
    win_size: (usize, usize),
    projection: Mat4,
    view: Mat4,
    color: Vec3,
    mesh: Mesh,
}

impl GeometryPass {
    pub fn new(id: RenderPassTargetId, mesh: Mesh, win_size: (usize, usize)) -> Self {
        GeometryPass {
            id,
            shader: None,
            texture: None,
            win_size,
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            // view: Mat4::from_scale(Vec3::new(scale as f32, scale as f32, scale as f32)),
            color: Vec3::new(1.0, 1.0, 1.0),
            mesh,
        }
    }

    fn update_projection(&mut self) {
        // let w = self.win_size.0 as f32;
        // let h = self.win_size.1 as f32;
        // self.projection = Mat4::orthographic_rh(
        //     -w / 2.0,
        //     w / 2.0,
        //     -h / 2.0,
        //     h / 2.0,
        //     -1.0,
        //     1.0,
        // );

        self.projection = Mat4::IDENTITY;
    }
}

impl RenderPass<CustomPassEvent> for GeometryPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_geometry_pass(ptr: *mut u8, event: &CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut GeometryPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_geometry_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: &CustomPassEvent) {
        match event {
            CustomPassEvent::ChangeColor(color) => {
                info!("Changing color to: {:?}", color);
                self.color = *color;
            }
            CustomPassEvent::UpdateShader(shader) => {
                info!("Updating shader: {:?}", shader);
                let clone = shader.clone();
                self.shader = Some(TriangleShaderContainer {
                    shader: clone,
                    model_location: shader.cast().get_uniform_location("model").unwrap(),
                    view_location: shader.cast().get_uniform_location("view").unwrap(),
                    proj_location: shader.cast().get_uniform_location("projection").unwrap(),
                    texture_uniform: shader
                        .cast()
                        .get_uniform_location("texture_diffuse1")
                        .unwrap(),
                });
            }
            CustomPassEvent::UpdateTexture(texture) => {
                info!("Updating texture: {:?}", texture);
                let clone = texture.clone();
                self.texture = Some(TextureContainer { texture: clone });
            }
            CustomPassEvent::UpdateWindowSize(width, height) => {
                info!("Updating window size: {}x{}", width, height);
                self.win_size = (*width, *height);
                self.update_projection();
            }
        }
    }

    fn name(&self) -> &str {
        "GeometryPass"
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> PassExecuteResult {
        // Nothing to do if no shader is set
        if self.shader.is_none() {
            return PassExecuteResult::default();
        }
        if self.texture.is_none() {
            return PassExecuteResult::default();
        }

        // Setup shader
        let shader_container = self.shader.as_ref().unwrap();
        let shader = shader_container.shader.cast();
        let shader_binding = shader.bind();
        shader_binding.set_uniform(shader_container.model_location, renderable.model);
        shader_binding.set_uniform(shader_container.view_location, self.view);
        shader_binding.set_uniform(shader_container.proj_location, self.projection);
        shader_binding.set_uniform(shader_container.texture_uniform, 0);

        // Setup texture
        let texture_container = self.texture.as_ref().unwrap();
        let texture = texture_container.texture.cast();
        let texture_binding = texture.bind(0);

        let binding = self.mesh.vao.bind();
        binding.draw_elements(self.mesh.count, DrawElementsMode::Triangles);

        PassExecuteResult::ok(1, 1)
    }
}

pub(crate) struct AABBPass {
    id: RenderPassTargetId,
    color: Vec3,
}
impl AABBPass {
    pub fn new(id: RenderPassTargetId) -> Self {
        AABBPass {
            id,
            color: Default::default(),
        }
    }
}

impl RenderPass<CustomPassEvent> for AABBPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_aabb_pass(ptr: *mut u8, event: &CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut AABBPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_aabb_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: &CustomPassEvent) {
        match event {
            CustomPassEvent::ChangeColor(color) => {
                info!("Changing color to: {:?}", color);
                self.color = *color;
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "AABBPass"
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> PassExecuteResult {
        PassExecuteResult::ok(0, 0)
    }
}
