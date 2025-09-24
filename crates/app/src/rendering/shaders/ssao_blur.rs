use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAOBlurShader {
    pub asset: TypedAsset<Program>,

    pub ubo_camera: u32,
    pub depth: UniformLocation,
    pub ssao_raw: UniformLocation,
    pub rough_occlusion_normal: UniformLocation,

    pub radius: UniformLocation,
    pub sigma_spatial: UniformLocation,
    pub sigma_depth: UniformLocation,
    pub sigma_normal: UniformLocation,
    pub ssao_enabled: UniformLocation,
}

impl SSAOBlurShader {
    pub fn new(
        shader: TypedAsset<Program>,
    ) -> Result<Self, dawn_graphics::gl::raii::shader::ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,

            ubo_camera: program.get_uniform_block_location("ubo_camera")?,

            depth: program.get_uniform_location("in_depth")?,
            ssao_raw: program.get_uniform_location("in_ssao_raw")?,
            rough_occlusion_normal: program.get_uniform_location("in_rough_occlusion_normal")?,
            ssao_enabled: program.get_uniform_location("in_ssao_enabled")?,

            radius: program.get_uniform_location("in_radius")?,
            sigma_spatial: program.get_uniform_location("in_sigma_spatial")?,
            sigma_depth: program.get_uniform_location("in_sigma_depth")?,
            sigma_normal: program.get_uniform_location("in_sigma_normal")?,
        })
    }
}
