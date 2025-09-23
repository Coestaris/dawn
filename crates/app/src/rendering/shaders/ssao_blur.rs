use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAOBlurShader {
    pub asset: TypedAsset<Program>,

    pub ubo_camera_location: u32,
    pub position_location: UniformLocation,
    pub ssao_raw_location: UniformLocation,
    pub normal_location: UniformLocation,
    
    pub radius: UniformLocation,
    pub sigma_spatial_location: UniformLocation,
    pub sigma_depth_location: UniformLocation,
    pub sigma_normal_location: UniformLocation,
    pub ssao_enabled: UniformLocation,
}

impl SSAOBlurShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, dawn_graphics::gl::raii::shader::ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
          
            position_location: program.get_uniform_location("in_position")?,
            ssao_raw_location: program.get_uniform_location("in_ssao_raw")?,
            normal_location: program.get_uniform_location("in_normal")?,
            ssao_enabled: program.get_uniform_location("in_ssao_enabled")?,
            
            radius: program.get_uniform_location("in_radius")?,
            sigma_spatial_location: program.get_uniform_location("in_sigma_spatial")?,
            sigma_depth_location: program.get_uniform_location("in_sigma_depth")?,
            sigma_normal_location: program.get_uniform_location("in_sigma_normal")?,
        })
    }
}