use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct PostprocessShader {
    pub asset: TypedAsset<Program>,
    pub fxaa_enabled: UniformLocation,
    pub texture_location: UniformLocation,
    pub ubo_camera_location: u32,
}

impl PostprocessShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            fxaa_enabled: program.get_uniform_location("in_fxaa_enabled")?,
            texture_location: program.get_uniform_location("in_texture")?,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
        })
    }
}
