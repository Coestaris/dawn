use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformBlockLocation, UniformLocation};

#[cfg(feature = "devtools")]
pub struct SSAOBlurShaderDevtools {
    pub tap_count: UniformLocation,
    pub sigma_depth: UniformLocation,
    pub ssao_enabled: UniformLocation,
    pub ubo_ssao_blur_taps: u32,
}

#[cfg(feature = "devtools")]
impl SSAOBlurShaderDevtools {
    pub fn new(program: &Program) -> Self {
        Self {
            tap_count: program.get_uniform_location("in_tap_count").unwrap(),
            sigma_depth: program.get_uniform_location("in_sigma_depth").unwrap(),
            ssao_enabled: program.get_uniform_location("in_ssao_enabled").unwrap(),
            ubo_ssao_blur_taps: program
                .get_uniform_block_location("ubo_ssao_blur_taps")
                .unwrap(),
        }
    }
}

pub struct SSAOBlurShader {
    pub asset: TypedAsset<Program>,

    pub ubo_camera: u32,
    pub stride: UniformLocation,
    pub halfres_ssao_raw: UniformLocation,
    pub halfres_normal: UniformLocation,
    pub halfres_depth: UniformLocation,

    #[cfg(feature = "devtools")]
    pub devtools: SSAOBlurShaderDevtools,
}

impl SSAOBlurShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,

            ubo_camera: program.get_uniform_block_location("ubo_camera")?,

            stride: program.get_uniform_location("in_stride")?,
            halfres_ssao_raw: program.get_uniform_location("in_halfres_ssao_raw")?,
            halfres_normal: program.get_uniform_location("in_halfres_normal")?,
            halfres_depth: program.get_uniform_location("in_halfres_depth")?,

            #[cfg(feature = "devtools")]
            devtools: SSAOBlurShaderDevtools::new(&program),
        })
    }
}
