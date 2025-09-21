use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAOBlurShader {
    pub asset: TypedAsset<Program>,
}

impl SSAOBlurShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, dawn_graphics::gl::raii::shader::ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
          
        })
    }
}