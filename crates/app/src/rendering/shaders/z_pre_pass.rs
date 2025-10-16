use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct ZPrepassShader {
    pub asset: TypedAsset<Program>,

    // Vertex uniforms
    pub ubo_camera_location: u32,
    pub model_location: UniformLocation,
}

impl ZPrepassShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            model_location: program.get_uniform_location("in_model")?,
        })
    }
}
