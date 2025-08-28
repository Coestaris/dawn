use crate::systems::rendering::CustomPassEvent;
use dawn_assets::ir::texture::{IRPixelDataType, IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::entities::material::Material;
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

fn create_missing_texture() -> Texture {
    // Create a 2x2 checkerboard pattern (magenta and black)
    let data: [u8; 12] = [
        255, 0, 255, // Magenta
        0, 0, 0, // Black
        0, 0, 0, // Black
        255, 0, 255, // Magenta
    ];

    let texture_ir = IRTexture {
        data: data.to_vec(),
        texture_type: IRTextureType::Texture2D {
            width: 2,
            height: 2,
        },
        pixel_format: IRPixelFormat::R8G8B8,
        use_mipmaps: false,
        min_filter: Default::default(),
        mag_filter: Default::default(),
        wrap_s: Default::default(),
        wrap_t: Default::default(),
        wrap_r: Default::default(),
    };

    Texture::from_ir::<CustomPassEvent>(texture_ir)
        .expect("Failed to create missing texture")
        .0
}

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
    missing_texture: Texture,
    projection: Mat4,
    view: Mat4,
    frame: usize,
}

impl GeometryPass {
    pub fn new(id: RenderPassTargetId, win_size: (usize, usize)) -> Self {
        GeometryPass {
            id,
            shader: None,
            win_size,
            missing_texture: create_missing_texture(),
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            frame: 0,
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
            program.bind();
            program.set_uniform(shader.proj_location, self.projection);
            program.set_uniform(shader.base_color_texture_location, 0);
            ShaderProgram::unbind();
        }

        unsafe {
            bindings::Viewport(0, 0, self.win_size.0 as i32, self.win_size.1 as i32);
            bindings::Scissor(0, 0, self.win_size.0 as i32, self.win_size.1 as i32);
        }
    }
}

impl RenderPass<CustomPassEvent> for GeometryPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_geometry_pass(ptr: *mut u8, event: CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut GeometryPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_geometry_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: CustomPassEvent) {
        match event {
            CustomPassEvent::UpdateShader(shader) => {
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
                self.win_size = (width, height);
                self.update_projection();
            }
            CustomPassEvent::UpdateView(view) => {
                self.view = view;
            }
            CustomPassEvent::DropAllAssets => {
                self.shader = None;
            }
        }
    }

    fn name(&self) -> &str {
        "GeometryPass"
    }

    #[inline(always)]
    fn begin(&mut self, _: &RendererBackend<CustomPassEvent>) -> PassExecuteResult {
        self.frame += 1;

        unsafe {
            bindings::ClearColor(0.1, 0.1, 0.1, 1.0);
            bindings::ClearDepth(1.0);
            bindings::Clear(bindings::COLOR_BUFFER_BIT | bindings::DEPTH_BUFFER_BIT);
        }

        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.shader.cast();
            program.bind();
            program.set_uniform(shader.view_location, self.view);
        }

        PassExecuteResult::default()
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<CustomPassEvent>) -> PassExecuteResult {
        ShaderProgram::unbind();
        Texture::unbind(bindings::TEXTURE_2D, 0);
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
            program.set_uniform(shader.model_location, renderable.model);
        }

        PassExecuteResult::default()
    }

    #[inline(always)]
    fn on_mesh(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        mesh: &Mesh,
    ) -> PassExecuteResult {
        if let None = self.shader {
            return PassExecuteResult::default();
        }

        mesh.draw(|submesh| {
            if let Some(material) = &submesh.material {
                let material = material.cast::<Material>();
                if let Some(texture) = &material.base_color_texture {
                    let texture = texture.cast::<Texture>();
                    texture.bind(0);
                } else {
                    // Bind a default white texture if no texture is set
                    self.missing_texture.bind(0);
                }
            } else {
                // Bind a default white texture if no texture is set
                self.missing_texture.bind(0);
            }

            (false, PassExecuteResult::default())
        })
    }
}
