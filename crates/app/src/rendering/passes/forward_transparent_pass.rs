use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::lighting::TransparentTarget;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::shaders::forward_transparent::ForwardTransparentShader;
use crate::rendering::textures::fallback_tex::FallbackTextures;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_assets::TypedAsset;
use dawn_graphics::gl::material::Material;
use dawn_graphics::gl::mesh::{Mesh, SubMesh, TopologyBucket};
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::texture::{Texture2D, TextureCube};
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::Mat4;
use glow::HasContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

const ALBEDO_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const METALLIC_ROUGHNESS_INDEX: i32 = 2;
const OCCLUSION_INDEX: i32 = 3;
const SKYBOX_INDEX: i32 = 4;

struct SortableSubmesh<'a, 'b> {
    model: Mat4,
    bucket: &'a TopologyBucket,
    submesh: &'b SubMesh,
    key: f32,
}

impl<'a, 'b> SortableSubmesh<'a, 'b> {
    fn new(model: Mat4, bucket: &'a TopologyBucket, submesh: &'b SubMesh) -> Self {
        Self {
            model,
            bucket,
            submesh,
            key: 0.0,
        }
    }

    fn sort_by_distance(view: Mat4, vector: &mut Vec<Self>) {
        // Calculate Keys
        for m in vector.iter_mut() {
            let min = view * m.model * m.submesh.min.extend(1.0);
            let max = view * m.model * m.submesh.max.extend(1.0);
            let center = (min + max) * 0.5;
            m.key = center.z;
        }

        // Sort by distance
        vector.sort_by(|a, b| a.key.partial_cmp(&b.key).unwrap());
    }

    // Assume shader already bound
    fn draw(&self, gl: &glow::Context, pass: &ForwardTransparentPass) -> RenderResult {
        #[cfg(feature = "devtools")]
        let tangents = if pass.config.get_force_no_tangents() {
            false
        } else {
            self.bucket.key.tangent_valid
        };
        #[cfg(not(feature = "devtools"))]
        let tangents = bucket.key.tangent_valid;

        let shader = pass.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        program.set_uniform(&shader.tangent_valid, tangents);
        program.set_uniform(&shader.model_location, self.model);

        let (albedo, normal, metallic_roughness, occlusion) =
            if let Some(material) = &self.submesh.material {
                let material = material.cast::<Material>();
                let albedo = material.albedo.cast();
                let normal = material.normal.cast();
                let metallic_roughness = material.metallic_roughness.cast();
                let occlusion = material.occlusion.cast();

                (albedo, normal, metallic_roughness, occlusion)
            } else {
                (
                    &pass.fallback_textures.albedo_texture,
                    &pass.fallback_textures.normal_texture,
                    &pass.fallback_textures.metallic_roughness_texture,
                    &pass.fallback_textures.occlusion_texture,
                )
            };

        Texture2D::bind(gl, albedo, ALBEDO_INDEX as u32);
        Texture2D::bind(gl, normal, NORMAL_INDEX as u32);
        Texture2D::bind(gl, metallic_roughness, METALLIC_ROUGHNESS_INDEX as u32);
        Texture2D::bind(gl, occlusion, OCCLUSION_INDEX as u32);

        let binding = self.bucket.vao.bind();
        binding.draw_elements_base_vertex(
            self.submesh.index_count,
            self.submesh.index_offset,
            self.submesh.vertex_offset,
        )
    }
}

pub(crate) struct ForwardTransparentPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ForwardTransparentShader>,
    fallback_textures: FallbackTextures,
    skybox: Option<TypedAsset<TextureCube>>,
    view: Option<Mat4>,

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
            view: None,
            frustum,
            target,
        }
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
                self.skybox = None;
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = Some(view);
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
        frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        Framebuffer::bind(&self.gl, &self.target.fbo);

        unsafe {
            // Correct depth information already in the G-Buffer
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Do not modify the depth buffer
            self.gl.depth_mask(false);

            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            if self.config.get_is_wireframe() {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
        }

        let shader = self.shader.as_ref().unwrap();
        let program = shader.asset.cast();
        Program::bind(&self.gl, &program);

        // Bind skybox if present
        if let Some(skybox) = &self.skybox {
            let skybox = skybox.cast();
            TextureCube::bind(&self.gl, skybox, SKYBOX_INDEX as u32);
        }

        // TODO: Do not reallocate this every frame
        let mut sortables = vec![];
        for renderable in &frame.renderables {
            let mesh = renderable.mesh.cast();

            // Check if the mesh is within the camera frustum
            // otherwise, skip rendering it at all
            if !self
                .frustum
                .borrow()
                .is_visible(mesh.min, mesh.max, renderable.model)
            {
                continue;
            }

            for bucket in &mesh.buckets {
                for submesh in &bucket.submesh {
                    if submesh.material.is_none() {
                        continue;
                    }

                    let material = submesh.material.as_ref().unwrap().cast::<Material>();
                    if !material.transparent {
                        continue;
                    }

                    sortables.push(SortableSubmesh::new(renderable.model, bucket, submesh))
                }
            }
        }

        SortableSubmesh::sort_by_distance(self.view.unwrap_or(Mat4::IDENTITY), &mut sortables);

        let mut result = RenderResult::default();
        for submesh in sortables {
            result += submesh.draw(&self.gl, self);
        }

        result
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
