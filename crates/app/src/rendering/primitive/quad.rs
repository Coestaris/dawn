use dawn_assets::ir::mesh::{
    IRIndexType, IRLayoutField, IRLayoutSampleType, IRMeshLayoutItem, IRTopology,
};
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::result::RenderResult;
use std::sync::Arc;

/// Defines the 2D circle primitive.
pub struct Quad2D {
    vao: VertexArray,
    _vbo: ArrayBuffer,
    _ebo: ElementArrayBuffer,
}

impl Quad2D {
    pub fn new(gl: Arc<glow::Context>) -> Self {
        let vertex: [f32; 16] = [
            // positions   // tex coords
            -1.0, 1.0, 0.0, 1.0, // top left
            -1.0, -1.0, 0.0, 0.0, // bottom left
            1.0, -1.0, 1.0, 0.0, // bottom right
            1.0, 1.0, 1.0, 1.0, // top right
        ];
        let indices_edges: [u16; 6] = [
            0, 1, 2, // first triangle
            0, 2, 3, // second triangle
        ];

        let vao = VertexArray::new(gl.clone(), IRTopology::Triangles, IRIndexType::U16).unwrap();
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
                samples: 2,
                stride_bytes: 16,
                offset_bytes: 0,
            },
        );
        vao_binding.setup_attribute(
            1,
            &IRMeshLayoutItem {
                field: IRLayoutField::TexCoord,
                sample_type: IRLayoutSampleType::Float,
                samples: 2,
                stride_bytes: 16,
                offset_bytes: 8,
            },
        );

        drop(vbo_binding);
        drop(ebo_binding);
        drop(vao_binding);

        Quad2D {
            vao,
            _vbo: vbo,
            _ebo: ebo,
        }
    }

    pub fn draw(&self) -> RenderResult {
        let binding = self.vao.bind();
        binding.draw_elements(6, 0)
    }
}
