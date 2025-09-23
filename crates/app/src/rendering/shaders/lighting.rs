use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

/// Optional part of the lighting shader.
/// Enabled only if the Devtools feature is toggled
pub struct LightingShaderDevtools {
    pub debug_mode: UniformLocation,
    pub sky_color_location: UniformLocation,
    pub ground_color_location: UniformLocation,
    pub diffuse_scale_location: UniformLocation,
    pub specular_scale_location: UniformLocation,
    pub ssao_enabled: UniformLocation,
}

impl LightingShaderDevtools {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let program = shader.cast();
        Ok(Self {
            debug_mode: program.get_uniform_location("in_debug_mode")?,
            sky_color_location: program.get_uniform_location("in_sky_color")?,
            ground_color_location: program.get_uniform_location("in_ground_color")?,
            diffuse_scale_location: program.get_uniform_location("in_diffuse_scale")?,
            specular_scale_location: program.get_uniform_location("in_specular_scale")?,
            ssao_enabled: program.get_uniform_location("in_ssao_enabled")?,
        })
    }
}

pub struct LightingShader {
    pub asset: TypedAsset<Program>,

    pub packed_lights_location: UniformLocation,
    pub packed_lights_header_location: UniformLocation,

    #[cfg(feature = "devtools")]
    pub devtools: LightingShaderDevtools,

    pub position_texture: UniformLocation,
    pub albedo_metallic_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub pbr_texture: UniformLocation,
    pub ssao_texture: UniformLocation,
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
            packed_lights_location: program.get_uniform_location("in_packed_lights")?,
            packed_lights_header_location: program
                .get_uniform_location("in_packed_lights_header")?,
            
            position_texture: program.get_uniform_location("in_position_texture")?,
            albedo_metallic_texture: program.get_uniform_location("in_albedo_metallic_texture")?,
            normal_texture: program.get_uniform_location("in_normal_texture")?,
            pbr_texture: program.get_uniform_location("in_pbr_texture")?,
            ssao_texture: program.get_uniform_location("in_ssao_texture")?,
        })
    }
}
