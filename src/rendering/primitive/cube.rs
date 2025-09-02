use dawn_assets::ir::mesh::{IRIndexType, IRLayout, IRLayoutField, IRLayoutSampleType, IRTopology};
use dawn_graphics::gl::raii::array_buffer::{ArrayBuffer, ArrayBufferUsage};
use dawn_graphics::gl::raii::element_array_buffer::{ElementArrayBuffer, ElementArrayBufferUsage};
use dawn_graphics::gl::raii::vertex_array::VertexArray;
use dawn_graphics::passes::result::RenderResult;
use glam::{Mat4, Vec3};

pub struct Cube {
    pub vao: VertexArray,
    pub vbo: ArrayBuffer,
    pub ebo: ElementArrayBuffer,
    pub indices_count: usize,
}

impl Cube {
    pub fn new() -> Self {
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

    pub fn draw(
        &self,
        mut set_model: impl FnMut(Mat4) -> (),
        min: Vec3,
        max: Vec3,
    ) -> RenderResult {
        // Calculate the translation matrix to transform the 1/1/1 cube to the min/max box
        let translation = Mat4::from_translation((min + max) / 2.0);
        let scale = Mat4::from_scale((max - min) / 2.0);
        // Note: The order of multiplication matters
        let model = translation * scale;

        set_model(model);

        let binding = self.vao.bind();
        binding.draw_elements(self.indices_count, 0)
    }
}
