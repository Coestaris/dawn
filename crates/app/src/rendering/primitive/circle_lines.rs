use dawn_assets::ir::mesh::{
    IRIndexType, IRLayoutField, IRLayoutSampleType, IRMeshLayoutItem, IRTopology,
};
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::result::RenderResult;
use std::sync::Arc;

/// Defines the 2D circle primitive as a line loop.
pub struct Circle3DLines {
    vao: VertexArray,
    _vbo: ArrayBuffer,
    _ebo: ElementArrayBuffer,
    index_count: usize,
}

impl Circle3DLines {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        const NUM_SEGMENTS: usize = 32;
        let mut vertex: Vec<f32> = Vec::new();
        let mut indices_edges: Vec<u16> = Vec::new();

        let step = 2.0 * std::f32::consts::PI / NUM_SEGMENTS as f32;
        let mut angle: f32 = 0.0;
        for i in 0..NUM_SEGMENTS {
            let x = angle.cos();
            let y = angle.sin();

            vertex.push(x); // position x
            vertex.push(y); // position y
            vertex.push(0.0);

            if i > 0 {
                indices_edges.push(i as u16);
                indices_edges.push((i - 1) as u16);
            }

            angle += step;
        }

        indices_edges.push(0);
        indices_edges.push((NUM_SEGMENTS - 1) as u16);

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
            &IRMeshLayoutItem {
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

        Circle3DLines {
            vao,
            _vbo: vbo,
            _ebo: ebo,
            index_count: indices_edges.len(),
        }
    }

    pub fn draw(&self) -> RenderResult {
        let binding = self.vao.bind();
        binding.draw_elements(self.index_count, 0)
    }
}
