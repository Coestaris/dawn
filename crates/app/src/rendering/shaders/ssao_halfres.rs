use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAOHalfresShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera: u32,
    pub depth: UniformLocation,
    pub albedo_metallic: UniformLocation,
    pub rough_occlusion_normal: UniformLocation,
}

impl SSAOHalfresShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera: program.get_uniform_block_location("ubo_camera")?,
            albedo_metallic: program.get_uniform_location("in_albedo_metallic")?,
            depth: program.get_uniform_location("in_depth")?,
            rough_occlusion_normal: program.get_uniform_location("in_rough_occlusion_normal")?,
        })
    }
}
