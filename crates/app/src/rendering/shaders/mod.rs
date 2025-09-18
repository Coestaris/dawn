use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub const LINE_SHADER: &str = "line_shader";
pub const GEOMETRY_SHADER: &str = "geometry_shader";
pub const BILLBOARD_SHADER: &str = "billboard_shader";
pub const LIGHTING_SHADER: &str = "lighting_shader";
pub const POSTPROCESS_SHADER: &str = "postprocess_shader";

pub struct LineShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera_location: u32,
    pub model_location: UniformLocation,
    pub color_location: UniformLocation,
}

impl LineShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            model_location: program.get_uniform_location("in_model")?,
            color_location: program.get_uniform_location("in_color")?,
        })
    }
}

pub struct GeometryShader {
    pub asset: TypedAsset<Program>,

    // Vertex uniforms
    pub ubo_camera_location: u32,
    pub model_location: UniformLocation,

    // Fragment uniforms
    pub albedo: UniformLocation,
    pub normal: UniformLocation,
    pub metallic_roughness: UniformLocation,
    pub occlusion: UniformLocation,
    pub tangent_valid: UniformLocation,
}

impl GeometryShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            model_location: program.get_uniform_location("in_model")?,
            albedo: program.get_uniform_location("in_albedo")?,
            normal: program.get_uniform_location("in_normal")?,
            metallic_roughness: program.get_uniform_location("in_metallic_roughness")?,
            occlusion: program.get_uniform_location("in_occlusion")?,
            tangent_valid: program.get_uniform_location("in_tangent_valid")?,
        })
    }
}

pub struct BillboardShader {
    pub asset: TypedAsset<Program>,
    pub ubo_camera_location: u32,
    pub texture_location: UniformLocation,
    pub size_location: UniformLocation,
    pub position_location: UniformLocation,
}

impl BillboardShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            texture_location: program.get_uniform_location("in_sprite")?,
            size_location: program.get_uniform_location("in_size")?,
            position_location: program.get_uniform_location("in_position")?,
            asset: clone,
        })
    }
}

/// Optional part of the lighting shader.
/// Enabled only if the Devtools feature is toggled
pub struct LightingShaderDevtools {
    pub debug_mode: UniformLocation,
    pub sky_color_location: UniformLocation,
    pub ground_color_location: UniformLocation,
    pub diffuse_scale_location: UniformLocation,
    pub specular_scale_location: UniformLocation,
}

impl LightingShaderDevtools {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let program = shader.cast();
        Ok(Self {
            debug_mode: program.get_uniform_location("in_debug_mode")?,
            sky_color_location: program.get_uniform_location("ENV_SKY_COLOR")?,
            ground_color_location: program.get_uniform_location("ENV_GROUND_COLOR")?,
            diffuse_scale_location: program.get_uniform_location("ENV_DIFFUSE_SCALE")?,
            specular_scale_location: program.get_uniform_location("ENV_SPECULAR_SCALE")?,
        })
    }
}

pub struct LightingShader {
    pub asset: TypedAsset<Program>,

    pub packed_lights_location: UniformLocation,
    pub packed_lights_header_location: UniformLocation,

    #[cfg(feature = "devtools")]
    pub devtools: LightingShaderDevtools,

    pub albedo_metallic_texture: UniformLocation,
    pub normal_texture: UniformLocation,
    pub pbr_texture: UniformLocation,
    pub depth_texture: UniformLocation,
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
            albedo_metallic_texture: program.get_uniform_location("in_albedo_metallic_texture")?,
            normal_texture: program.get_uniform_location("in_normal_texture")?,
            pbr_texture: program.get_uniform_location("in_pbr_texture")?,
            depth_texture: program.get_uniform_location("in_depth_texture")?,
        })
    }
}

pub struct PostprocessShader {
    pub asset: TypedAsset<Program>,
    pub fxaa_enabled: UniformLocation,
    pub texture_location: UniformLocation,
    pub ubo_camera_location: u32,
}

impl PostprocessShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            fxaa_enabled: program.get_uniform_location("fxaa_enabled")?,
            texture_location: program.get_uniform_location("in_texture")?,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
        })
    }
}
