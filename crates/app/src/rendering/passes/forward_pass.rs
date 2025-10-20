use crate::rendering::bind_tracker::TextureBindTracker;
use crate::rendering::config::RenderingConfig;
use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::gbuffer::GBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::shaders::forward::ForwardShader;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_graphics::gl::material::Material;
use dawn_graphics::gl::mesh::{SubMesh, TopologyBucket};
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::vertex_array::VertexArray;
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

const ALBEDO_INDEX: i32 = 0;
const NORMAL_INDEX: i32 = 1;
const METALLIC_ROUGHNESS_INDEX: i32 = 2;
const OCCLUSION_INDEX: i32 = 3;

pub(crate) struct ForwardPass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,
    config: RenderingConfig,

    shader: Option<ForwardShader>,

    frustum: Rc<RefCell<FrustumCulling>>,
    tbt: TextureBindTracker<5>,

    gbuffer: Rc<GBuffer>,
}

impl ForwardPass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<GBuffer>,
        frustum: Rc<RefCell<FrustumCulling>>,
        config: RenderingConfig,
    ) -> Self {
        ForwardPass {
            gl: gl.clone(),
            id,
            config,
            shader: None,
            frustum,
            tbt: TextureBindTracker::new(),
            gbuffer,
        }
    }

    fn prepare_bucket(&self, bucket: &TopologyBucket) {
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
    }

    fn prepare_submesh(&mut self, model: &Mat4, submesh: &SubMesh) -> bool {
        if !self
            .frustum
            .borrow()
            .is_visible(submesh.min, submesh.max, *model)
        {
            return false;
        }

        // Check if the submesh at the camera frustum
        // otherwise, skip rendering
        // TODO: Is it worth to do frustum culling per submesh?
        if let Some(material) = &submesh.material {
            let material = material.cast::<Material>();

            // Transparent submeshes are not rendered
            // They will be rendered in a separate pass
            if material.transparent {
                return false;
            }

            let albedo = material.albedo.cast();
            let normal = material.normal.cast();
            let metallic_roughness = material.metallic_roughness.cast();
            let occlusion = material.occlusion.cast();

            self.tbt.bind2d(&self.gl, ALBEDO_INDEX, albedo);
            self.tbt.bind2d(&self.gl, NORMAL_INDEX, normal);
            self.tbt
                .bind2d(&self.gl, METALLIC_ROUGHNESS_INDEX, metallic_roughness);
            self.tbt.bind2d(&self.gl, OCCLUSION_INDEX, occlusion);
        };

        return true;
    }
}

impl RenderPass<RenderingEvent> for ForwardPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut ForwardPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::UpdateShader(_, shader) => {
                self.shader = Some(ForwardShader::new(shader.clone()).unwrap());

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
                Program::unbind(&self.gl);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "ForwardPass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &Window,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        Framebuffer::bind(&self.gl, &self.gbuffer.fbo);

        unsafe {
            // Clear color
            self.gl.clear(glow::COLOR_BUFFER_BIT);

            // Correct depth information already in the G-Buffer
            self.gl.depth_func(glow::EQUAL);
            // Do not modify the depth buffer
            self.gl.depth_mask(false);

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

        let mut result = RenderResult::default();
        for bucket in &mesh.buckets {
            self.prepare_bucket(bucket);

            VertexArray::bind(&self.gl, &bucket.vao);
            for submesh in &bucket.submesh {
                if !self.prepare_submesh(&renderable.model, submesh) {
                    continue;
                }

                result += bucket.vao.draw_elements_base_vertex(
                    submesh.index_count,
                    submesh.index_offset,
                    submesh.vertex_offset,
                );
            }
            VertexArray::unbind(&self.gl);
        }

        result
    }

    #[inline(always)]
    fn end(&mut self, _: &Window, _: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        unsafe {
            self.gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
            self.gl.depth_mask(true);
        }

        Program::unbind(&self.gl);
        self.tbt.unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        RenderResult::default()
    }
}
