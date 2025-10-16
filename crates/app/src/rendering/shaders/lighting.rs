use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

/// Optional part of the lighting shader.
/// Enabled only if the Devtools feature is toggled
pub struct LightingShaderDevtools {
    pub debug_mode: UniformLocation,
    pub diffuse_scale_location: UniformLocation,
    pub specular_scale_location: UniformLocation,
    pub ssao_enabled: UniformLocation,
}

impl LightingShaderDevtools {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let program = shader.cast();
        Ok(Self {
            debug_mode: program.get_uniform_location("in_debug_mode")?,
            diffuse_scale_location: program.get_uniform_location("in_diffuse_scale")?,
            specular_scale_location: program.get_uniform_location("in_specular_scale")?,
            ssao_enabled: program.get_uniform_location("in_ssao_enabled")?,
        })
    }
}

pub struct LightingShader {
    pub asset: TypedAsset<Program>,

    pub packed_lights: UniformLocation,
    pub packed_lights_header: UniformLocation,

    #[cfg(feature = "devtools")]
    pub devtools: LightingShaderDevtools,

    pub depth: UniformLocation,
    pub albedo: UniformLocation,
    pub orm: UniformLocation,
    pub normal: UniformLocation,
    pub halfres_ssao: UniformLocation,
    pub skybox: UniformLocation,
}

impl LightingShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone1 = shader.clone();
        let clone2 = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone1,
            #[cfg(feature = "devtools")]
            devtools: LightingShaderDevtools::new(clone2)?,
            packed_lights: program.get_uniform_location("in_packed_lights")?,
            packed_lights_header: program.get_uniform_location("in_packed_lights_header")?,

            depth: program.get_uniform_location("in_depth")?,
            albedo: program.get_uniform_location("in_albedo")?,
            orm: program.get_uniform_location("in_orm")?,
            normal: program.get_uniform_location("in_normal")?,
            halfres_ssao: program.get_uniform_location("in_halfres_ssao")?,
            skybox: program.get_uniform_location("in_skybox")?,
        })
    }
}
