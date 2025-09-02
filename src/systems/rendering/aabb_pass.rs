use crate::systems::rendering::gbuffer::GBuffer;
use crate::systems::rendering::CustomPassEvent;
use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use dawn_assets::TypedAsset;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::framebuffer::{
    BlitFramebufferFilter, BlitFramebufferMask, Framebuffer,
};
use dawn_graphics::gl::raii::shader_program::{ShaderProgram, UniformLocation};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::events::{PassEventTarget, RenderPassTargetId};
use dawn_graphics::passes::result::RenderResult;
use dawn_graphics::passes::RenderPass;
use dawn_graphics::renderable::Renderable;
use dawn_graphics::renderer::RendererBackend;
use glam::{Mat4, UVec2, Vec3, Vec4};
use log::debug;
use std::rc::Rc;

struct Cube {
    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
    pub ebo: ElementArrayBuffer,
    pub indices_count: usize,
}

impl Cube {
    fn new() -> Self {
        let vertex = [
            // Top face
            1.0f32, 1.0, 1.0, // 0
            1.0, 1.0, -1.0, // 1
            -1.0, 1.0, -1.0, // 2
            -1.0, 1.0, 1.0, // 3
            // Bottom face
            1.0, -1.0, 1.0, // 4
            1.0, -1.0, -1.0, // 5
            -1.0, -1.0, -1.0, // 6
            -1.0, -1.0, 1.0, // 7
        ];

        let indices_edges = [
            0u16, 1, 1, 2, 2, 3, 3, 0, // Top face edges
            4, 5, 5, 6, 6, 7, 7, 4, // Bottom face edges
            0, 4, 1, 5, 2, 6, 3, 7, // Side edges
        ];
        let vao = VertexArray::new(IRTopology::Lines, IRIndexType::U16).unwrap();
        let mut vbo = ArrayBuffer::new().unwrap();
        let mut ebo = ElementArrayBuffer::new().unwrap();

        let vao_binding = vao.bind();
        let vbo_binding = vbo.bind();
        let ebo_binding = ebo.bind();

        vbo_binding.feed(&vertex, ArrayBufferUsage::StaticDraw);
        ebo_binding.feed(&indices_edges, ElementArrayBufferUsage::StaticDraw);

        vao_binding.setup_attribute(
            0,
            &IRLayout {
                field: IRLayoutField::Position,
                sample_type: IRLayoutSampleType::Float,
                samples: 3,
                stride_bytes: 12,
                offset_bytes: 0,
            },
        );

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        Cube {
            vao,
            vbo,
            ebo,
            indices_count: indices_edges.len(),
        }
    }

    fn draw(
        &self,
        shader: &ShaderContainer,
        color: Vec4,
        model: Mat4,
        min: Vec3,
        max: Vec3,
    ) -> RenderResult {
        // Assume shader is already bound
        let program = shader.shader.cast();

        // Assume projection and view matrices are already set

        // Calculate the translation matrix to transform the 1/1/1 cube to the min/max box
        let translation = Mat4::from_translation((min + max) / 2.0);
        let scale = Mat4::from_scale((max - min) / 2.0);
        // Note: The order of multiplication matters
        let model = model * translation * scale;

        program.set_uniform(shader.model_location, model);
        program.set_uniform(shader.color_location, color);

        let binding = self.vao.bind();
        binding.draw_elements(self.indices_count, 0)
    }
}

struct ShaderContainer {
    shader: TypedAsset<ShaderProgram>,
    model_location: UniformLocation,
    view_location: UniformLocation,
    proj_location: UniformLocation,
    color_location: UniformLocation,
}

#[derive(Debug)]
pub enum AABBMode {
    Disable,
    IgnoreDepthBuffer,
    RespectDepthBuffer,
}

impl AABBMode {
    fn cycle(&mut self) {
        *self = match self {
            AABBMode::Disable => AABBMode::IgnoreDepthBuffer,
            AABBMode::IgnoreDepthBuffer => AABBMode::RespectDepthBuffer,
            AABBMode::RespectDepthBuffer => AABBMode::Disable,
        }
    }
}

pub(crate) struct AABBPass {
    id: RenderPassTargetId,
    cube: Cube,
    mode: AABBMode,
    shader: Option<ShaderContainer>,
    projection: Mat4,
    usize: UVec2,
    view: Mat4,
    gbuffer: Rc<GBuffer>,
}

impl AABBPass {
    pub fn new(id: RenderPassTargetId, gbuffer: Rc<GBuffer>) -> Self {
        AABBPass {
            id,
            shader: None,
            cube: Cube::new(),
            projection: Mat4::IDENTITY,
            usize: UVec2::ZERO,
            view: Mat4::IDENTITY,
            mode: AABBMode::Disable,
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
            ShaderProgram::bind(&program);
            program.set_uniform(shader.proj_location, self.projection);
            ShaderProgram::unbind();
        }
    }
}

impl RenderPass<CustomPassEvent> for AABBPass {
    fn get_target(&self) -> Vec<PassEventTarget<CustomPassEvent>> {
        fn dispatch_aabb_pass(ptr: *mut u8, event: CustomPassEvent) {
            let pass = unsafe { &mut *(ptr as *mut AABBPass) };
            pass.dispatch(event);
        }

        vec![PassEventTarget::new(dispatch_aabb_pass, self.id, self)]
    }

    fn dispatch(&mut self, event: CustomPassEvent) {
        match event {
            CustomPassEvent::UpdateShader(shader) => {
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
            CustomPassEvent::UpdateWindowSize(size) => {
                self.usize = size;
                self.calculate_projection(size);
                self.set_projection();
            }
            CustomPassEvent::ToggleAABB => {
                self.mode.cycle();
                debug!("AABBPass mode: {:?}", self.mode);
            }
            CustomPassEvent::UpdateView(view) => {
                self.view = view;
            }
            CustomPassEvent::DropAllAssets => {
                self.shader = None;
            }
            _ => {}
        }
    }

    fn name(&self) -> &str {
        "AABBPass"
    }

    fn begin(&mut self, _backend: &RendererBackend<CustomPassEvent>) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }

        match self.mode {
            AABBMode::Disable => {
                return RenderResult::default();
            }
            AABBMode::IgnoreDepthBuffer => {}
            AABBMode::RespectDepthBuffer => {
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
        }

        // Bind shader
        let shader = self.shader.as_ref().unwrap();
        let program = shader.shader.cast();
        ShaderProgram::bind(&program);

        // Update view
        program.set_uniform(shader.view_location, self.view);

        RenderResult::default()
    }

    #[inline(always)]
    fn on_renderable(
        &mut self,
        _: &mut RendererBackend<CustomPassEvent>,
        renderable: &Renderable,
    ) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if matches!(self.mode, AABBMode::Disable) {
            return RenderResult::default();
        }

        let mesh = renderable.mesh.cast();
        let shader = self.shader.as_ref().unwrap();
        let mut result = RenderResult::default();
        result += self.cube.draw(
            shader,
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            renderable.model,
            mesh.min,
            mesh.max,
        );

        for bucket in &mesh.buckets {
            for submesh in &bucket.submesh {
                result += self.cube.draw(
                    shader,
                    Vec4::new(0.0, 1.0, 0.0, 1.0),
                    renderable.model,
                    submesh.min,
                    submesh.max,
                );
            }
        }

        result
    }

    fn end(&mut self, _backend: &mut RendererBackend<CustomPassEvent>) -> RenderResult {
        if self.shader.is_none() {
            return RenderResult::default();
        }
        if matches!(self.mode, AABBMode::Disable) {
            return RenderResult::default();
        }

        // Unbind shader
        ShaderProgram::unbind();

        RenderResult::default()
    }
}
