use crate::rendering::event::RenderingEvent;
use crate::rendering::frustum::FrustumCulling;
use crate::rendering::gbuffer::GBuffer;
use crate::rendering::primitive::cube::Cube;
use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::framebuffer::{
    BlitFramebufferFilter, BlitFramebufferMask, Framebuffer,
};
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::{DataStreamFrame, RendererBackend};
use glam::{Mat4, UVec2, Vec3, Vec4};
use log::debug;
use std::rc::Rc;

struct ShaderContainer {
    shader: TypedAsset<Program>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
    color_location: UniformLocation,
}

#[derive(Debug)]
enum Mode {
    Disabled,
    AABBRespectDepthBuffer,
    OBBRespectDepthBuffer,
    OBBIgnoreDepthBuffer,
    AABBIgnoreDepthBuffer,
}

impl Mode {
    fn cycle(&mut self) {
        *self = match self {
            Mode::Disabled => Mode::AABBRespectDepthBuffer,
            Mode::AABBRespectDepthBuffer => Mode::OBBRespectDepthBuffer,
            Mode::OBBRespectDepthBuffer => Mode::OBBIgnoreDepthBuffer,
            Mode::OBBIgnoreDepthBuffer => Mode::AABBIgnoreDepthBuffer,
            Mode::AABBIgnoreDepthBuffer => Mode::Disabled,
        }
    }
}

pub(crate) struct BoundingPass {
    id: RenderPassTargetId,
    cube: Cube,
    mode: Mode,
    shader: Option<ShaderContainer>,
    projection: Mat4,
    usize: UVec2,
    view: Mat4,
    gbuffer: Rc<GBuffer>,
}

impl BoundingPass {
    pub fn new(id: RenderPassTargetId, gbuffer: Rc<GBuffer>) -> Self {
        BoundingPass {
            id,
            shader: None,
            cube: Cube::new(),
            projection: Mat4::IDENTITY,
            usize: UVec2::ZERO,
            view: Mat4::IDENTITY,
            mode: Mode::Disabled,
            gbuffer,
        }
    }

    fn calculate_projection(&mut self, win_size: UVec2) {
        self.projection = Mat4::perspective_rh(
            45.0f32.to_radians(),
            win_size.x as f32 / win_size.y as f32,
            0.1,
            100.0,
        );
    }

    fn set_projection(&mut self) {
        if let Some(shader) = self.shader.as_mut() {
            // Load projection matrix into shader
            let program = shader.shader.cast();
            Program::bind(&program);
            program.set_uniform(shader.proj_location, self.projection);
            Program::unbind();
        }
    }
}

impl RenderPass<RenderingEvent> for BoundingPass {
    fn get_target(&self) -> Vec<PassEventTarget<RenderingEvent>> {
        fn dispatch_bounding_pass(ptr: *mut u8, event: RenderingEvent) {
            let pass = unsafe { &mut *(ptr as *mut BoundingPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_bounding_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: RenderingEvent) {
        match event {
            RenderingEvent::DropAllAssets => {
                self.shader = None;
            }
            RenderingEvent::UpdateShader(shader) => {
                let clone = shader.clone();
                let casted = shader.cast();
                self.shader = Some(ShaderContainer {
                    shader: clone,
                    model_location: casted.get_uniform_location("model").unwrap(),
                    view_location: casted.get_uniform_location("view").unwrap(),
                    proj_location: casted.get_uniform_location("projection").unwrap(),
                    color_location: casted.get_uniform_location("color").unwrap(),
                });
                self.set_projection();
            }
            RenderingEvent::ViewportResized(size) => {
                self.usize = size;
            }
            RenderingEvent::PerspectiveProjectionUpdated(proj) => {
                self.projection = proj;
                self.set_projection();
            }
            RenderingEvent::ViewUpdated(view) => {
                self.view = view;
            }

            RenderingEvent::ToggleBoundingBox => {
                self.mode.cycle();
                debug!("Bounding Box mode: {:?}", self.mode);
            }

            _ => {}
        }
    }

    fn name(&self) -> &str {
        "BoundingBox"
    }

    fn begin(
        &mut self,
        _backend: &RendererBackend<RenderingEvent>,
        _frame: &DataStreamFrame,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        match self.mode {
            Mode::Disabled => {
                return RenderResult::default();
            }
            Mode::AABBRespectDepthBuffer | Mode::OBBRespectDepthBuffer => {
                // Blit the depth buffer to the default framebuffer
                Framebuffer::blit_to_default(
                    &self.gbuffer.fbo,
                    self.usize,
                    BlitFramebufferMask::Depth,
                    BlitFramebufferFilter::Nearest,
                );

                // Enable depth test
                unsafe {
                    bindings::Enable(bindings::DEPTH_TEST);
                    bindings::DepthFunc(bindings::LEQUAL);
                }
            }
            _ => {}
        }

        // Bind shader
        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        Program::bind(&program);

        // Update view
        program.set_uniform(shader.view_location, self.view);

        RenderResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<RenderingEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if matches!(self.mode, Mode::Disabled) {
            return RenderResult::default();
        }

        let mesh = renderable.mesh.cast();
        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        let mut result = RenderResult::default();

        static MESH_COLOR: Vec4 = Vec4::new(1.0, 0.0, 0.0, 1.0);
        static SUBMESH_COLOR: Vec4 = Vec4::new(0.0, 1.0, 0.0, 1.0);

        fn draw_cube(
            pass: &BoundingPass,
            renderable_model: Mat4,
            min: Vec3,
            max: Vec3,
        ) -> RenderResult {
            let shader = pass.shader.as_ref().unwrap();
            let program = shader.shader.cast();

            if matches!(
                pass.mode,
                Mode::OBBIgnoreDepthBuffer | Mode::OBBRespectDepthBuffer
            ) {
                pass.cube.draw(
                    |model| {
                        let obb = renderable_model * model;
                        program.set_uniform(shader.model_location, obb);
                    },
                    min,
                    max,
                )
            } else {
                let (min, max) = FrustumCulling::obb_to_aabb(min, max, renderable_model);
                pass.cube.draw(
                    |model| {
                        program.set_uniform(shader.model_location, model);
                    },
                    min,
                    max,
                )
            }
        }

        program.set_uniform(shader.color_location, MESH_COLOR);
        result += draw_cube(self, renderable.model, mesh.min, mesh.max);

        program.set_uniform(shader.color_location, SUBMESH_COLOR);
        for bucket in &mesh.buckets {
            for submesh in &bucket.submesh {
                result += draw_cube(self, renderable.model, submesh.min, submesh.max);
            }
        }

        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<RenderingEvent>) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if matches!(self.mode, Mode::Disabled) {
            return RenderResult::default();
        }

        // Unbind shader
        Program::unbind();

        RenderResult::default()
    }
}
