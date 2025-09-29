use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAORawShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera: u32,
    pub ubo_ssao_raw_params: u32,
    pub ubo_ssao_raw_kernel: u32,

    pub depth: UniformLocation,
    pub rough_occlusion_normal: UniformLocation,
}

impl SSAORawShader {
    pub fn new(
        shader: TypedAsset<Program>,
    ) -> Result<Self, dawn_graphics::gl::raii::shader::ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera: program.get_uniform_block_location("ubo_camera")?,
            ubo_ssao_raw_params: program.get_uniform_block_location("ubo_ssao_raw_params")?,
            ubo_ssao_raw_kernel: program.get_uniform_block_location("ubo_ssao_raw_kernel")?,
            depth: program.get_uniform_location("in_depth")?,
            rough_occlusion_normal: program.get_uniform_location("in_rough_occlusion_normal")?,
        })
    }
}
