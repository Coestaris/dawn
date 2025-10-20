use crate::rendering::event::RenderingEvent;
use crate::rendering::fbo::dbuffer::DBuffer;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::shaders::z_pre_pass::ZPrepassShader;
use crate::rendering::ubo::camera::CameraUBO;
use crate::rendering::ubo::CAMERA_UBO_BINDING;
use dawn_graphics::gl::material::Material;
use dawn_graphics::gl::mesh::SubMesh;
use dawn_graphics::gl::raii::framebuffer::Framebuffer;
use dawn_graphics::gl::raii::shader_program::Program;
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{Mat4, UVec2};
use glow::HasContext;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use winit::window::Window;

pub(crate) struct ZPrePass {
    gl: Arc<glow::Context>,
    id: RenderPassTargetId,

    shader: Option<ZPrepassShader>,
    viewport: UVec2,

    frustum: Rc<RefCell<FrustumCulling>>,

    dbuffer: Rc<DBuffer>,
    camera_ubo: CameraUBO,
}

impl ZPrePass {
    pub fn new(
        gl: Arc<glow::Context>,
        id: RenderPassTargetId,
        gbuffer: Rc<DBuffer>,
        frustum: Rc<RefCell<FrustumCulling>>,
    ) -> Self {
        ZPrePass {
            gl: gl.clone(),
            id,
            shader: None,
            viewport: Default::default(),
            frustum,
            dbuffer: gbuffer,
            camera_ubo: CameraUBO::new(gl.clone(), CAMERA_UBO_BINDING),
        }
    }

    fn prepare_submesh(&self, model: &Mat4, submesh: &SubMesh) -> bool {
        // Check if the submesh at the camera frustum
        // otherwise, skip rendering
        // TODO: Is it worth to do frustum culling per submesh?
        if !self
            .frustum
            .borrow()
            .is_visible(submesh.min, submesh.max, *model)
        {
            return false;
        }

        if let Some(material) = &submesh.material {
            let material = material.cast::<Material>();

            // Transparent submeshes are not rendered
            // They will be rendered in a separate pass
            if material.transparent {
                return false;
            }
        }

        true
    }
}

impl RenderPass<RenderingEvent> for ZPrePass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut ZPrePass) };
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
                self.shader = Some(ZPrepassShader::new(shader.clone()).unwrap());

                // Setup shader static uniforms
                let shader = self.shader.as_ref().unwrap();
                let program = shader.asset.cast();
                Program::bind(&self.gl, &program);
                program.set_uniform_block_binding(
                    shader.ubo_camera_location,
                    CAMERA_UBO_BINDING as u32,
                );
                Program::unbind(&self.gl);
            }

            RenderingEvent::ViewportResized(size) => {
                self.viewport = size;
                self.camera_ubo.set_viewport(size.x as f32, size.y as f32);
                self.dbuffer.resize(size);
                self.camera_ubo.upload();
            }

            RenderingEvent::PerspectiveProjectionUpdated(proj, near, far) => {
                self.frustum.borrow_mut().set_perspective(proj);
                self.camera_ubo.set_perspective(proj, near, far);
                self.camera_ubo.upload();
            }
            RenderingEvent::ViewUpdated(view) => {
                self.frustum.borrow_mut().set_view(view);
                self.camera_ubo.set_view(view);
                self.camera_ubo.upload();
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "ZPrePass"
    }

    #[inline(always)]
    fn begin(
        &mut self,
        _: &Window,
        _: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        unsafe {
            // Setup viewport
            self.gl
                .viewport(0, 0, self.viewport.x as i32, self.viewport.y as i32);
            self.gl
                .scissor(0, 0, self.viewport.x as i32, self.viewport.y as i32);
        }

        Framebuffer::bind(&self.gl, &self.dbuffer.fbo);

        unsafe {
            // Setup clear color and depth
            self.gl.clear(glow::DEPTH_BUFFER_BIT);

            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LESS);
            // Enable depth writing
            self.gl.depth_mask(true);

            // Rendering only the opaque objects
            self.gl.disable(glow::BLEND);
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
        Program::unbind(&self.gl);
        Framebuffer::unbind(&self.gl);
        RenderResult::default()
    }
}
