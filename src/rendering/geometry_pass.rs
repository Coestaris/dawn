use crate::rendering::CustomPassEvent;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::entities::mesh::Mesh;
use dawn_graphics::gl::entities::shader_program::{ShaderProgram, UniformLocation};
use dawn_graphics::gl::entities::texture::Texture;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::PassExecuteResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::RendererBackend;
use glam::{Mat4, Vec3};
use log::info;

struct TriangleShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
    base_color_texture_location: UniformLocation,
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
            binding.set_uniform(shader.base_color_texture_location, 0);
        }

        unsafe {
            bindings::Viewport(0, 0, self.win_size.0 as i32, self.win_size.1 as i32);
            bindings::Scissor(0, 0, self.win_size.0 as i32, self.win_size.1 as i32);
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
                    base_color_texture_location: shader
                        .cast()
                        .get_uniform_location("base_color_texture")
                        .unwrap(),
                });
                self.update_projection();
            }
            CustomPassEvent::UpdateWindowSize(width, height) => {
                self.win_size = (*width, *height);
                self.update_projection();
            }
            CustomPassEvent::UpdateView(view) => {
                self.view = *view;
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
        }

        let material = renderable.material.cast();
        if let Some(texture) = &material.base_color_texture {
            let texture = texture.cast::<Texture>();
            let _binding = texture.bind(0);
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
