use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::shaders::forward::ForwardShader;
use crate::rendering::textures::fallback_tex::FallbackTextures;
use crate::rendering::ubo::camera::CameraUBO;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_graphics::gl::material::Material;
use dawn_graphics::gl::mesh::{SubMesh, TopologyBucket};
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture2D, TextureCube};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::Mat4;
use glow::HasContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;
use dawn_assets::TypedAsset;
use crate::rendering::fbo::lighting::TransparentTarget;
use crate::rendering::shaders::forward_transparent::ForwardTransparentShader;

const ALBEDO_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const METALLIC_ROUGHNESS_INDEX: i32 = 2;
const OCCLUSION_INDEX: i32 = 3;
const SKYBOX_INDEX: i32 = 4;

pub(crate) struct ForwardTransparentPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ForwardTransparentShader>,
    fallback_textures: FallbackTextures,
    skybox: Option<TypedAsset<TextureCube>>,

    frustum: Rc<RefCell<FrustumCulling>>,
    target: TransparentTarget,
}

impl ForwardTransparentPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        target: TransparentTarget,
        frustum: Rc<RefCell<FrustumCulling>>,
        config: RenderingConfig,
    ) -> Self {
        ForwardTransparentPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            fallback_textures: FallbackTextures::new(gl),
            skybox: None,
            frustum,
            target,
        }
    }

    fn prepare_bucket(&self, bucket: &TopologyBucket) -> (bool, RenderResult) {
        #[cfg(feature = "devtools")]
        let tangents = if self.config.get_force_no_tangents() {
            false
        } else {
            bucket.key.tangent_valid
        };
        #[cfg(not(feature = "devtools"))]
        let tangents = bucket.key.tangent_valid;

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        program.set_uniform(&shader.tangent_valid, tangents);
        (false, RenderResult::default())
    }

    fn prepare_submesh(&self, model: &Mat4, submesh: &SubMesh) -> (bool, RenderResult) {
        // Check if the submesh at the camera frustum
        // otherwise, skip rendering
        // TODO: Is it worth to do frustum culling per submesh?
        if !self
            .frustum
            .borrow()
            .is_visible(submesh.min, submesh.max, *model)
        {
            return (true, RenderResult::default());
        }

        let (albedo, normal, metallic_roughness, occlusion) =
            if let Some(material) = &submesh.material {
                let material = material.cast::<Material>();

                if !material.transparent {
                    return (true, RenderResult::default());
                }

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

        Texture2D::bind(&self.gl, albedo, ALBEDO_INDEX as u32);
        Texture2D::bind(&self.gl, normal, NORMAL_INDEX as u32);
        Texture2D::bind(
            &self.gl,
            metallic_roughness,
            METALLIC_ROUGHNESS_INDEX as u32,
        );
        Texture2D::bind(&self.gl, occlusion, OCCLUSION_INDEX as u32);

        (false, RenderResult::default())
    }
}

impl RenderPass<RenderingEvent> for ForwardTransparentPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut ForwardTransparentPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::SetSkybox(skybox) => {
                self.skybox = Some(skybox);
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(ForwardTransparentShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                program.set_uniform(&shader.albedo, ALBEDO_INDEX);
                program.set_uniform(&shader.normal, NORMAL_INDEX);
                program.set_uniform(&shader.metallic_roughness, METALLIC_ROUGHNESS_INDEX);
                program.set_uniform(&shader.occlusion, OCCLUSION_INDEX);
                program.set_uniform(&shader.skybox, SKYBOX_INDEX);
                Program::unbind(&self.gl);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "ForwardTransparentPass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &Window,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        Framebuffer::bind(&self.gl, &self.target.fbo);

        unsafe {
            // Correct depth information already in the G-Buffer
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Do not modify the depth buffer
            self.gl.depth_mask(false);

            self.gl.enable(glow::BLEND);
            self.gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            if self.config.get_is_wireframe() {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
        }

        if let Some(shader) = self.shader.as_mut() {
            // Load view matrix into shader
            let program = shader.asset.cast();
            Program::bind(&self.gl, &program);
        }

        RenderResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &Window,
        _: &mut RendererBackend<RenderingEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        let mesh = renderable.mesh.cast();

        // Check if the mesh is within the camera frustum
        // otherwise, skip rendering it at all
        if !self
            .frustum
            .borrow()
            .is_visible(mesh.min, mesh.max, renderable.model)
        {
            return RenderResult::default();
        }

        let shader = self.shader.as_mut().unwrap();
        let program = shader.asset.cast();

        program.set_uniform(&shader.model_location, renderable.model);
        if let Some(skybox) = &self.skybox {
            let skybox = skybox.cast();
            TextureCube::bind(&self.gl, skybox, SKYBOX_INDEX as u32);
        }

        // TODO: Sort transparent submeshes by distance to camera

        mesh.draw(
            |bucket| self.prepare_bucket(bucket),
            |submesh| self.prepare_submesh(&renderable.model, submesh),
        )
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        unsafe {
            self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            self.gl.depth_mask(true);
            self.gl.disable(glow::BLEND);
        }

        Program::unbind(&self.gl);
        Texture2D::unbind(&self.gl, ALBEDO_INDEX as u32);
        Texture2D::unbind(&self.gl, NORMAL_INDEX as u32);
        Texture2D::unbind(&self.gl, METALLIC_ROUGHNESS_INDEX as u32);
        Texture2D::unbind(&self.gl, OCCLUSION_INDEX as u32);
        Framebuffer::unbind(&self.gl);
        RenderResult::default()
    }
}
