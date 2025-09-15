use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fallback_tex::FallbackTextures;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::ubo::camera::CameraUBO;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
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
use glow::HasContext;
use std::rc::Rc;
use std::sync::Arc;

const ALBEDO_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const METALLIC_ROUGHNESS_INDEX: i32 = 2;
const OCCLUSION_INDEX: i32 = 3;

struct ShaderContainer {
    shader: TypedAsset<Program>,

    // Vertex uniforms
    ubo_camera_location: u32,
    model_location: UniformLocation,

    // Fragment uniforms
    albedo: UniformLocation,
    normal: UniformLocation,
    metallic_roughness: UniformLocation,
    occlusion: UniformLocation,
}

pub(crate) struct GeometryPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ShaderContainer>,
    fallback_textures: FallbackTextures,

    frustum: FrustumCulling,

    gbuffer: Rc<GBuffer>,
    camera_ubo: CameraUBO,
}

impl GeometryPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        camera_ubo: CameraUBO,
        config: RenderingConfig,
    ) -> Self {
        GeometryPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            fallback_textures: FallbackTextures::new(gl),
            frustum: FrustumCulling::new(),
            gbuffer,
            camera_ubo,
        }
    }
}

impl RenderPass<RenderingEvent> for GeometryPass {
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
                    ubo_camera_location: shader.get_uniform_block_location("ubo_camera").unwrap(),
                    model_location: shader.get_uniform_location("in_model").unwrap(),
                    albedo: shader.get_uniform_location("in_albedo").unwrap(),
                    normal: shader.get_uniform_location("in_normal").unwrap(),
                    metallic_roughness: shader
                        .get_uniform_location("in_metallic_roughness")
                        .unwrap(),
                    occlusion: shader.get_uniform_location("in_occlusion").unwrap(),
                });

                if let Some(shader) = self.shader.as_mut() {
                    let program = shader.shader.cast();
                    Program::bind(&self.gl, &program);
                    program.set_uniform_block_binding(
                        shader.ubo_camera_location,
                        CAMERA_UBO_BINDING as u32,
                    );
                    program.set_uniform(&shader.albedo, ALBEDO_INDEX);
                    program.set_uniform(&shader.normal, NORMAL_INDEX);
                    program.set_uniform(&shader.metallic_roughness, METALLIC_ROUGHNESS_INDEX);
                    program.set_uniform(&shader.occlusion, OCCLUSION_INDEX);
                    Program::unbind(&self.gl);
                }
            }
            RenderingEvent::ViewportResized(size) => unsafe {
                self.gl.viewport(0, 0, size.x as i32, size.y as i32);
                self.gl.scissor(0, 0, size.x as i32, size.y as i32);
                self.camera_ubo.set_viewport(size.x as f32, size.y as f32);
                self.camera_ubo.upload();
            },
            RenderingEvent::PerspectiveProjectionUpdated(proj) => {
                self.frustum.set_perspective(proj);
                self.camera_ubo.set_perspective(proj);
                self.camera_ubo.upload();
            }
            RenderingEvent::ViewUpdated(view) => {
                self.frustum.set_view(view);
                self.camera_ubo.set_view(view);
                self.camera_ubo.upload();
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
        Framebuffer::bind(&self.gl, &self.gbuffer.fbo);

        unsafe {
            self.gl.clear_color(0.1, 0.1, 0.1, 1.0);
            self.gl.clear_depth(1.0);
            self.gl
                .clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.enable(glow::CULL_FACE);
            self.gl.cull_face(glow::BACK);
            self.gl.depth_func(glow::LESS);
            self.gl.disable(glow::BLEND);

            if self.config.get_is_wireframe() {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
        }

        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.shader.cast();
            Program::bind(&self.gl, &program);
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
            program.set_uniform(&shader.model_location, renderable.model);

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

                let (albedo, normal, metallic_roughness, occlusion) =
                    if let Some(material) = &submesh.material {
                        let material = material.cast::<Material>();

                        let albedo = material.albedo.cast();
                        let normal = material.normal.cast();
                        let metallic_roughness = material.metallic_roughness.cast();
                        let occlusion = material.occlusion.cast();

                        (albedo, normal, metallic_roughness, occlusion)
                    } else {
                        (
                            &self.fallback_textures.albedo_texture,
                            &self.fallback_textures.normal_texture,
                            &self.fallback_textures.metallic_roughness_texture,
                            &self.fallback_textures.occlusion_texture,
                        )
                    };

                Texture::bind(
                    &self.gl,
                    TextureBind::Texture2D,
                    albedo,
                    ALBEDO_INDEX as u32,
                );
                Texture::bind(
                    &self.gl,
                    TextureBind::Texture2D,
                    normal,
                    NORMAL_INDEX as u32,
                );
                Texture::bind(
                    &self.gl,
                    TextureBind::Texture2D,
                    metallic_roughness,
                    METALLIC_ROUGHNESS_INDEX as u32,
                );
                Texture::bind(
                    &self.gl,
                    TextureBind::Texture2D,
                    occlusion,
                    OCCLUSION_INDEX as u32,
                );

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

        Program::unbind(&self.gl);
        Texture::unbind(&self.gl, TextureBind::Texture2D, ALBEDO_INDEX as u32);
        Texture::unbind(&self.gl, TextureBind::Texture2D, NORMAL_INDEX as u32);
        Texture::unbind(
            &self.gl,
            TextureBind::Texture2D,
            METALLIC_ROUGHNESS_INDEX as u32,
        );
        Texture::unbind(&self.gl, TextureBind::Texture2D, OCCLUSION_INDEX as u32);
        Framebuffer::unbind(&self.gl);
        RenderResult::default()
    }
}
