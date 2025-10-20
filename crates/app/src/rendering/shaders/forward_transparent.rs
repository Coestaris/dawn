use dawn_assets::TypedAsset;
use dawn_graphics::gl::raii::shader::ShaderError;
use dawn_graphics::gl::raii::shader_program::{Program, UniformLocation};

pub struct ForwardTransparentShaderDevtools {
    pub diffuse_scale: UniformLocation,
    pub specular_scale: UniformLocation,
}

impl ForwardTransparentShaderDevtools {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            diffuse_scale: program.get_uniform_location("in_diffuse_scale")?,
            specular_scale: program.get_uniform_location("in_specular_scale")?,
        })
    }
}

pub struct ForwardTransparentShader {
    pub asset: TypedAsset<Program>,

    // Vertex uniforms
    pub ubo_camera_location: u32,
    pub model_location: UniformLocation,
    pub packed_lights: UniformLocation,
    pub packed_lights_header: UniformLocation,

    // Fragment uniforms
    pub albedo: UniformLocation,
    pub normal: UniformLocation,
    pub metallic_roughness: UniformLocation,
    pub occlusion: UniformLocation,
    pub tangent_valid: UniformLocation,
    pub skybox: UniformLocation,
}

impl ForwardTransparentShader {
    pub fn new(shader: TypedAsset<Program>) -> Result<Self, ShaderError> {
        let clone = shader.clone();
        let program = shader.cast();
        Ok(Self {
            asset: clone,
            ubo_camera_location: program.get_uniform_block_location("ubo_camera")?,
            model_location: program.get_uniform_location("in_model")?,
            packed_lights: program.get_uniform_location("in_packed_lights")?,
            packed_lights_header: program.get_uniform_location("in_packed_lights_header")?,
            albedo: program.get_uniform_location("in_albedo")?,
            normal: program.get_uniform_location("in_normal")?,
            metallic_roughness: program.get_uniform_location("in_metallic_roughness")?,
            occlusion: program.get_uniform_location("in_occlusion")?,
            tangent_valid: program.get_uniform_location("in_tangent_valid")?,
            skybox: program.get_uniform_location("in_skybox")?,
        })
    }
}
