use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct BillboardShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera_location: u32,
    pub texture_location: UniformLocation,
    pub size_location: UniformLocation,
    pub position_location: UniformLocation,
}

impl BillboardShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            texture_location: program.get_uniform_location("in_sprite")?,
            size_location: program.get_uniform_location("in_size")?,
            position_location: program.get_uniform_location("in_position")?,
            asset: clone,
        })
    }
}