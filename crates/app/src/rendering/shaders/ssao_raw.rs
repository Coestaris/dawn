use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct SSAORawShaderDevtools {
    pub kernel_size: UniformLocation,
    pub radius: UniformLocation,
    pub bias: UniformLocation,
    pub intensity: UniformLocation,
    pub power: UniformLocation,
    pub ssao_enabled: UniformLocation,
    pub ubo_ssao_raw_kernel: u32,
}

impl SSAORawShaderDevtools {
    pub fn new(program: &Program) -> Self {
        Self {
            ubo_ssao_raw_kernel: program
                .get_uniform_block_location("ubo_ssao_raw_kernel")
                .unwrap(),
            kernel_size: program.get_uniform_location("in_kernel_size").unwrap(),
            radius: program.get_uniform_location("in_radius").unwrap(),
            bias: program.get_uniform_location("in_bias").unwrap(),
            intensity: program.get_uniform_location("in_intensity").unwrap(),
            power: program.get_uniform_location("in_power").unwrap(),
            ssao_enabled: program.get_uniform_location("in_ssao_enabled").unwrap(),
        }
    }
}

pub struct SSAORawShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera: u32,

    pub halfres_depth: UniformLocation,
    pub halfres_normal: UniformLocation,
    #[cfg(feature = "devtools")]
    pub devtools: SSAORawShaderDevtools,
}

impl SSAORawShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera: program.get_uniform_block_location("ubo_camera")?,
            halfres_depth: program.get_uniform_location("in_halfres_depth")?,
            halfres_normal: program.get_uniform_location("in_halfres_normal")?,
            #[cfg(feature = "devtools")]
            devtools: SSAORawShaderDevtools::new(&program),
        })
    }
}
