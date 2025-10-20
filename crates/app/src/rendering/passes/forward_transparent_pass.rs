use crate::rendering::bind_tracker::{TextureBindTracker, VAOBindTracker};
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
use dawn_graphics::gl::raii::vertex_array::VertexArray;
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

#[derive(Clone)]
struct Transparent {
    model: Mat4,
    renderable_idx: usize,
    bucket_idx: usize,
    submesh_idx: usize,
}

type SortKey = f32;

impl Transparent {
    fn new(model: Mat4, renderable_idx: usize, bucket_idx: usize, submesh_idx: usize) -> Self {
        Transparent {
            model,
            renderable_idx,
            bucket_idx,
            submesh_idx,
        }
    }

    pub fn get_key(&self, view: Mat4, mesh: &Mesh) -> SortKey {
        let bucket = &mesh.buckets[self.bucket_idx];
        let submesh = &bucket.submesh[self.submesh_idx];

        let c_obj = (submesh.min + submesh.max) * 0.5;
        let ext = (submesh.max - submesh.min) * 0.5;
        let radius = ext.length();

        let c_vs = (view * self.model * c_obj.extend(1.0)).z;

        let key = c_vs - radius;

        // If the key is not finite,
        // then the object is behind the camera
        if !key.is_finite() {
            f32::INFINITY
        } else {
            key
        }
    }

    // Assume shader already bound
    fn draw(
        &self,
        gl: &glow::Context,
        config: &RenderingConfig,
        shader: &ForwardTransparentShader,
        mesh: &Mesh,
        tbt: &mut TextureBindTracker<5>,
        vbt: &mut VAOBindTracker,
    ) -> RenderResult {
        let bucket = &mesh.buckets[self.bucket_idx];
        let submesh = &bucket.submesh[self.submesh_idx];

        #[cfg(feature = "devtools")]
        let tangents = if config.get_force_no_tangents() {
            false
        } else {
            bucket.key.tangent_valid
        };
        #[cfg(not(feature = "devtools"))]
        let tangents = bucket.key.tangent_valid;

        let program = shader.asset.cast();
        program.set_uniform(&shader.tangent_valid, tangents);
        program.set_uniform(&shader.model_location, self.model);

        if let Some(material) = &submesh.material {
            let material = material.cast::<Material>();
            let albedo = material.albedo.cast();
            let normal = material.normal.cast();
            let metallic_roughness = material.metallic_roughness.cast();
            let occlusion = material.occlusion.cast();

            tbt.bind2d(gl, ALBEDO_INDEX, albedo);
            tbt.bind2d(gl, NORMAL_INDEX, normal);
            tbt.bind2d(gl, METALLIC_ROUGHNESS_INDEX, metallic_roughness);
            tbt.bind2d(gl, OCCLUSION_INDEX, occlusion);
        }

        vbt.bind(gl, &bucket.vao);
        let result = bucket.vao.draw_elements_base_vertex(
            submesh.index_count,
            submesh.index_offset,
            submesh.vertex_offset,
        );
        result
    }
}

pub(crate) struct ForwardTransparentPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ForwardTransparentShader>,
    skybox: Option<TypedAsset<TextureCube>>,
    view: Option<Mat4>,

    frustum: Rc<RefCell<FrustumCulling>>,
    target: TransparentTarget,

    keys_buffer: Vec<SortKey>,
    shuffle_buffer: Vec<usize>,
    transparent_buffer: Vec<Transparent>,

    tbt: TextureBindTracker<5>,
    vbt: VAOBindTracker,
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
            skybox: None,
            view: None,
            frustum,
            target,

            keys_buffer: Vec::with_capacity(1024),
            shuffle_buffer: Vec::with_capacity(1024),
            transparent_buffer: Vec::with_capacity(1024),
            tbt: TextureBindTracker::new(),
            vbt: VAOBindTracker::new(),
        }
    }

    fn prepare_transparent(&mut self, frame: &DataStreamFrame) {
        // Clear buffers
        self.keys_buffer.clear();
        self.shuffle_buffer.clear();
        self.transparent_buffer.clear();

        // Collect all transparent submeshes
        let mut idx = 0;
        for (renderable_idx, renderable) in frame.renderables.iter().enumerate() {
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

            for (bucket_idx, bucket) in mesh.buckets.iter().enumerate() {
                for (submesh_idx, submesh) in bucket.submesh.iter().enumerate() {
                    if submesh.material.is_none() {
                        continue;
                    }

                    let material = submesh.material.as_ref().unwrap().cast::<Material>();
                    if !material.transparent {
                        continue;
                    }

                    let transparent =
                        Transparent::new(renderable.model, renderable_idx, bucket_idx, submesh_idx);
                    self.keys_buffer
                        .push(transparent.get_key(self.view.unwrap_or(Mat4::IDENTITY), mesh));
                    self.transparent_buffer.push(transparent);
                    self.shuffle_buffer.push(idx);
                    idx += 1;
                }
            }
        }

        // Sort indices by keys from keys buffer
        self.shuffle_buffer.sort_unstable_by(|a, b| {
            let i = *a;
            let j = *b;

            let ord = self.keys_buffer[i].total_cmp(&self.keys_buffer[j]);
            if ord.is_eq() {
                // Try to sort by renderable, bucket and submesh indices
                self.transparent_buffer[i]
                    .renderable_idx
                    .cmp(&self.transparent_buffer[j].renderable_idx)
                    .then(
                        self.transparent_buffer[i]
                            .bucket_idx
                            .cmp(&self.transparent_buffer[j].bucket_idx),
                    )
                    .then(
                        self.transparent_buffer[i]
                            .submesh_idx
                            .cmp(&self.transparent_buffer[j].submesh_idx),
                    )
            } else {
                ord
            }
        });
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
            self.gl.depth_func(glow::LEQUAL);
            // Do not modify the depth buffer
            self.gl.depth_mask(false);

            self.gl.enable(glow::BLEND);
            self.gl
                .blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);

            if self.config.get_is_wireframe() {
                self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
            }
        }

        // Make rust happy about the borrowing of self.shader
        {
            let shader = self.shader.as_ref().unwrap();
            let program = shader.asset.cast();
            Program::bind(&self.gl, &program);

            // Bind skybox if present
            if let Some(skybox) = &self.skybox {
                let skybox = skybox.cast();
                self.tbt.bind_cube(&self.gl, SKYBOX_INDEX, skybox);
            }
        }

        self.prepare_transparent(frame);

        let shader = self.shader.as_ref().unwrap();
        let mut result = RenderResult::default();
        for idx in &self.shuffle_buffer {
            let transparent = &self.transparent_buffer[*idx];
            let renderable = &frame.renderables[transparent.renderable_idx];
            let mesh = renderable.mesh.cast();
            result += transparent.draw(
                &self.gl,
                &self.config,
                shader,
                mesh,
                &mut self.tbt,
                &mut self.vbt,
            );
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
        self.tbt.unbind(&self.gl);
        self.vbt.unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        RenderResult::default()
    }
}
