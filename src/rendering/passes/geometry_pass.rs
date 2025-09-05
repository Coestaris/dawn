use crate::rendering::event::RenderingEvent;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::gbuffer::GBuffer;
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::material::Material;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::Mat4;
use glow::HasContext;
use std::rc::Rc;

fn create_missing_texture(gl: &glow::Context) -> Texture<'_> {
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
        pixel_format: IRPixelFormat::RGB8,
        use_mipmaps: false,
        min_filter: Default::default(),
        mag_filter: Default::default(),
        wrap_s: Default::default(),
        wrap_t: Default::default(),
        wrap_r: Default::default(),
    };

    Texture::from_ir::<RenderingEvent>(gl, texture_ir)
        .expect("Failed to create missing texture")
        .0
}

struct ShaderContainer {
    shader: TypedAsset<Program<'static>>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
    base_color_texture_location: UniformLocation,
}

pub(crate) struct GeometryPass<'g> {
    gl: &'g glow::Context,
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    missing_texture: Texture<'g>,
    projection: Mat4,
    view: Mat4,
    is_wireframe: bool,
    frustum: FrustumCulling,
    gbuffer: Rc<GBuffer<'g>>,
}

impl<'g> GeometryPass<'g> {
    pub fn new(gl: &'g glow::Context, id: RenderPassTargetId, gbuffer: Rc<GBuffer<'g>>) -> Self {
        GeometryPass {
            gl,
            id,
            shader: None,
            missing_texture: create_missing_texture(gl),
            projection: Mat4::IDENTITY,
            view: Mat4::IDENTITY,
            is_wireframe: false,
            frustum: FrustumCulling::new(Mat4::IDENTITY, Mat4::IDENTITY),
            gbuffer,
        }
    }

    fn set_projection(&mut self) {
        if let Some(shader) = self.shader.as_mut() {
            // Load projection matrix into shader
            let program = shader.shader.cast();
            Program::bind(self.gl, &program);
            program.set_uniform(shader.proj_location, self.projection);
            program.set_uniform(shader.base_color_texture_location, 0);
            Program::unbind(self.gl);
        }
    }
}

impl<'g> RenderPass<RenderingEvent> for GeometryPass<'g> {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_geometry_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut GeometryPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_geometry_pass, self.id, self)]
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
                    model_location: shader.cast().get_uniform_location("model").unwrap(),
                    view_location: shader.cast().get_uniform_location("view").unwrap(),
                    proj_location: shader.cast().get_uniform_location("projection").unwrap(),
                    base_color_texture_location: shader
                        .cast()
                        .get_uniform_location("base_color_texture")
                        .unwrap(),
                });
                self.set_projection();
            }
            RenderingEvent::ViewportResized(size) => unsafe {
                self.gl.viewport(0, 0, size.x as i32, size.y as i32);
                self.gl.scissor(0, 0, size.x as i32, size.y as i32);
            },
            RenderingEvent::PerspectiveProjectionUpdated(proj) => {
                self.projection = proj;
                self.frustum = FrustumCulling::new(self.projection, self.view);
                self.set_projection();
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = view;
                self.frustum = FrustumCulling::new(self.projection, self.view);
            }

            RenderingEvent::ToggleWireframeMode => {
                self.is_wireframe = !self.is_wireframe;
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "GeometryPass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        Framebuffer::bind(self.gl, &self.gbuffer.fbo);

        unsafe {
            self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            self.gl.clear_depth(1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            if self.is_wireframe {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
        }

        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.shader.cast();
            Program::bind(self.gl, &program);
            program.set_uniform(shader.view_location, self.view);
        }

        RenderResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<RenderingEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        if let Some(shader) = self.shader.as_mut() {
            let mesh = renderable.mesh.cast();

            // Check if the mesh is within the camera frustum
            // otherwise, skip rendering it at all
            if !self
                .frustum
                .is_visible(mesh.min, mesh.max, renderable.model)
            {
                return RenderResult::default();
            }

            // Load view matrix into shader
            let program = shader.shader.cast();
            program.set_uniform(shader.model_location, renderable.model);

            mesh.draw(|submesh| {
                // Check if the submesh at the camera frustum
                // otherwise, skip rendering
                // TODO: Is it worth to do frustum culling per submesh?
                if !self
                    .frustum
                    .is_visible(submesh.min, submesh.max, renderable.model)
                {
                    return (true, RenderResult::default());
                }

                let base_color = if let Some(material) = &submesh.material {
                    let material = material.cast::<Material>();
                    if let Some(texture) = &material.base_color_texture {
                        let texture = texture.cast::<Texture>();
                        texture
                    } else {
                        // Bind a default white texture if no texture is set
                        &self.missing_texture
                    }
                } else {
                    // Bind a default white texture if no texture is set
                    &self.missing_texture
                };

                Texture::bind(self.gl, TextureBind::Texture2D, base_color, 0);

                (false, RenderResult::default())
            })
        } else {
            RenderResult::default()
        }
    }

    #[inline(always)]
    fn end(&mut self, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        unsafe {
            self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
        }

        Program::unbind(self.gl);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
        Framebuffer::unbind(self.gl);
        RenderResult::default()
    }
}
