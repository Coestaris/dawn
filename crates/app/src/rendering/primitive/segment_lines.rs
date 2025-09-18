use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::result::RenderResult;
use glam::{Mat4, Vec3};
use std::sync::Arc;

/// Defines the 2-point line primitive as a line loop along the z-axis.
pub struct Segment3DLines {
    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
    pub ebo: ElementArrayBuffer,
}

impl Segment3DLines {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        let vertex = [0.0f32, 0.0, 0.0, 0.0, 0.0, 1.0];
        let indices_edges = [0u16, 1];

        let vao = VertexArray::new(gl.clone(), IRTopology::Lines, IRIndexType::U16).unwrap();
        let mut vbo = ArrayBuffer::new(gl.clone()).unwrap();
        let mut ebo = ElementArrayBuffer::new(gl).unwrap();

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

        Segment3DLines { vao, vbo, ebo }
    }

    pub fn draw(&self) -> RenderResult {
        let binding = self.vao.bind();
        binding.draw_elements(2, 0)
    }
}
