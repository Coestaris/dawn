use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAORawShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera_location: u32,
    pub ubo_ssao_raw_params_location: u32,
    pub ubo_ssao_raw_kernel_location: u32,

    pub depth_location: UniformLocation,
    pub normal_location: UniformLocation,
    pub noise_location: UniformLocation,
    pub albedo_location: UniformLocation,
}

impl SSAORawShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, dawn_graphics::gl::raii::shader::ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            ubo_ssao_raw_params_location: program.get_uniform_block_location("ubo_ssao_raw_params")?,
            ubo_ssao_raw_kernel_location: program.get_uniform_block_location("ubo_ssao_raw_kernel")?,
            depth_location: program.get_uniform_location("in_depth")?,
            normal_location: program.get_uniform_location("in_normal")?,
            noise_location: program.get_uniform_location("in_noise")?,
            albedo_location: program.get_uniform_location("in_albedo")?,
        })
    }
}