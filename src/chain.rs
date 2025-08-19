use dawn_assets::TypedAsset;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::gl::entities::shader_program::ShaderProgram;
use dawn_graphics::gl::entities::shader_program::UniformLocation;
use dawn_graphics::gl::entities::texture::Texture;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::PassExecuteResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::RendererBackend;
use glam::{Mat4, Vec3};
use log::info;

#[derive(Debug, Clone)]
pub(crate) enum CustomPassEvent {
    UpdateShader(TypedAsset<ShaderProgram>),
    UpdateWindowSize(usize, usize),
}

struct TriangleShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
}

struct TextureContainer {
    texture: TypedAsset<Texture>,
}

pub(crate) struct GeometryPass {
    id: RenderPassTargetId,
    shader: Option<TriangleShaderContainer>,
    win_size: (usize, usize),
    projection: Mat4,
    view: Mat4,
    rotation: f32,
    color: Vec3,
}

impl GeometryPass {
    pub fn new(id: RenderPassTargetId, win_size: (usize, usize)) -> Self {
        GeometryPass {
            id,
            shader: None,
            win_size,
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            rotation: 0.0,
            color: Vec3::new(1.0, 1.0, 1.0),
        }
    }

    fn update_projection(&mut self) {
        self.projection = Mat4::perspective_rh(
            45.0f32.to_radians(),
            self.win_size.0 as f32 / self.win_size.1 as f32,
            0.1,
            100.0,
        );

        if let Some(shader) = self.shader.as_mut() {
            // Load projection matrix into shader
            let program = shader.shader.cast();
            let binding = program.bind();
            binding.set_uniform(shader.proj_location, self.projection);
        }
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
            CustomPassEvent::UpdateShader(shader) => {
                info!("Updating shader: {:?}", shader);
                let clone = shader.clone();
                self.shader = Some(TriangleShaderContainer {
                    shader: clone,
                    model_location: shader.cast().get_uniform_location("model").unwrap(),
                    view_location: shader.cast().get_uniform_location("view").unwrap(),
                    proj_location: shader.cast().get_uniform_location("projection").unwrap(),
                });
                self.update_projection();
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
    fn begin(&mut self, _backend: &RendererBackend<CustomPassEvent>) -> PassExecuteResult {
        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.shader.cast();
            let binding = program.bind();
            binding.set_uniform(shader.view_location, self.view);
        }

        PassExecuteResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> PassExecuteResult {
        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.shader.cast();
            let binding = program.bind();
            binding.set_uniform(shader.model_location, renderable.model);

            // TODO: Setup textures
        }

        PassExecuteResult::default()
    }

    #[inline(always)]
    fn on_mesh(
        &mut self,
        _backend: &mut RendererBackend<CustomPassEvent>,
        mesh: &Mesh,
    ) -> PassExecuteResult {
        mesh.draw()
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
