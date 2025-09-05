use crate::rendering::event::RenderingEvent;
use crate::rendering::fallback_tex::FallbackTextures;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::gbuffer::GBuffer;
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

const ALBEDO_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const METALLIC_INDEX: i32 = 2;
const ROUGHNESS_INDEX: i32 = 3;
const OCCLUSION_INDEX: i32 = 4;

struct ShaderContainer {
    shader: TypedAsset<Program<'static>>,

    // Vertex uniforms
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,

    // Fragment uniforms
    albedo: UniformLocation,
    normal: UniformLocation,
    metallic: UniformLocation,
    roughness: UniformLocation,
    occlusion: UniformLocation,
}

pub(crate) struct GeometryPass<'g> {
    gl: &'g glow::Context,
    id: RenderPassTargetId,
    shader: Option<ShaderContainer>,
    fallback_textures: FallbackTextures<'g>,
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
            fallback_textures: FallbackTextures::new(gl),
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

            program.set_uniform(shader.albedo, ALBEDO_INDEX);
            program.set_uniform(shader.normal, NORMAL_INDEX);
            program.set_uniform(shader.metallic, METALLIC_INDEX);
            program.set_uniform(shader.roughness, ROUGHNESS_INDEX);
            program.set_uniform(shader.occlusion, OCCLUSION_INDEX);

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
                let shader = shader.cast();
                self.shader = Some(ShaderContainer {
                    shader: clone,

                    model_location: shader.get_uniform_location("in_model").unwrap(),
                    view_location: shader.get_uniform_location("in_view").unwrap(),
                    proj_location: shader.get_uniform_location("in_projection").unwrap(),

                    albedo: shader.get_uniform_location("in_albedo").unwrap(),
                    normal: shader.get_uniform_location("in_normal").unwrap(),
                    metallic: shader.get_uniform_location("in_metallic").unwrap(),
                    roughness: shader.get_uniform_location("in_roughness").unwrap(),
                    occlusion: shader.get_uniform_location("in_occlusion").unwrap(),
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

                let (albedo, normal, metallic, roughness, occlusion) =
                    if let Some(material) = &submesh.material {
                        let material = material.cast::<Material>();

                        let albedo = if let Some(texture) = &material.base_color_texture {
                            let texture = texture.cast::<Texture>();
                            texture
                        } else {
                            &self.fallback_textures.albedo_texture
                        };

                        // TODO: Get normal texture from material
                        let normal = &self.fallback_textures.normal_texture;

                        let metallic = if let Some(texture) = &material.metallic_texture {
                            let texture = texture.cast::<Texture>();
                            texture
                        } else {
                            &self.fallback_textures.metallic_texture
                        };

                        let roughness = if let Some(texture) = &material.roughness_texture {
                            let texture = texture.cast::<Texture>();
                            texture
                        } else {
                            &self.fallback_textures.roughness_texture
                        };

                        // TODO: Get occlusion texture from material
                        let occlusion = &self.fallback_textures.occlusion_texture;

                        (albedo, normal, metallic, roughness, occlusion)
                    } else {
                        (
                            &self.fallback_textures.albedo_texture,
                            &self.fallback_textures.normal_texture,
                            &self.fallback_textures.metallic_texture,
                            &self.fallback_textures.roughness_texture,
                            &self.fallback_textures.occlusion_texture,
                        )
                    };

                Texture::bind(self.gl, TextureBind::Texture2D, albedo, ALBEDO_INDEX as u32);
                Texture::bind(self.gl, TextureBind::Texture2D, normal, NORMAL_INDEX as u32);
                Texture::bind(self.gl, TextureBind::Texture2D, metallic, METALLIC_INDEX as u32);
                Texture::bind(self.gl, TextureBind::Texture2D, roughness, ROUGHNESS_INDEX as u32);
                Texture::bind(self.gl, TextureBind::Texture2D, occlusion, OCCLUSION_INDEX as u32);

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
        Texture::unbind(self.gl, TextureBind::Texture2D, ALBEDO_INDEX as u32);
        Texture::unbind(self.gl, TextureBind::Texture2D, NORMAL_INDEX as u32);
        Texture::unbind(self.gl, TextureBind::Texture2D, METALLIC_INDEX as u32);
        Texture::unbind(self.gl, TextureBind::Texture2D, ROUGHNESS_INDEX as u32);
        Texture::unbind(self.gl, TextureBind::Texture2D, OCCLUSION_INDEX as u32);
        Framebuffer::unbind(self.gl);
        RenderResult::default()
    }
}
