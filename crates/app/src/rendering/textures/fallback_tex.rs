use crate::rendering::event::RenderingEvent;
use dawn_assets::ir::texture2d::{IRPixelFormat, IRTexture2D};
use dawn_graphics::gl::raii::texture::Texture2D;
use std::sync::Arc;

pub struct FallbackTextures {
    pub albedo_texture: Texture2D,
    pub normal_texture: Texture2D,
    pub metallic_roughness_texture: Texture2D,
    pub occlusion_texture: Texture2D,
}

impl FallbackTextures {
    pub(crate) fn new(gl: Arc<glow::Context>) -> Self {
        let albedo_texture = Self::create_missing_albedo_texture(gl.clone());
        let normal_texture = Self::create_missing_normal_texture(gl.clone());
        let metallic_roughness_texture =
            Self::create_missing_metallic_roughness_texture(gl.clone());
        let occlusion_texture = Self::create_missing_occlusion_texture(gl.clone());

        FallbackTextures {
            albedo_texture,
            normal_texture,
            metallic_roughness_texture,
            occlusion_texture,
        }
    }

    fn create_missing_albedo_texture(gl: Arc<glow::Context>) -> Texture2D {
        // Create a 2x2 checkerboard pattern (magenta and black)
        let data: [u8; 12] = [
            255, 0, 255, // Magenta
            0, 0, 0, // Black
            0, 0, 0, // Black
            255, 0, 255, // Magenta
        ];

        let texture_ir = IRTexture2D {
            data: data.to_vec(),
            width: 2,
            height: 2,
            pixel_format: IRPixelFormat::RGB8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
        };

        Texture2D::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_normal_texture(gl: Arc<glow::Context>) -> Texture2D {
        let data = [
            128u8, 128, 255, 128, 128, 255, 128, 128, 255, 128, 128, 255, // Row 1
            128u8, 128, 255, 128, 128, 255, 128, 128, 255, 128, 128, 255, // Row 2
        ];

        let texture_ir = IRTexture2D {
            data: data.to_vec(),
            width: 2,
            height: 2,
            pixel_format: IRPixelFormat::RGB8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
        };

        Texture2D::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_metallic_roughness_texture(gl: Arc<glow::Context>) -> Texture2D {
        let data = [
            0u8, 255, 0, 255, // Row 1
            0u8, 255, 0, 255, // Row 2
        ];

        let texture_ir = IRTexture2D {
            data: data.to_vec(),
            width: 2,
            height: 2,
            pixel_format: IRPixelFormat::RG8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
        };

        Texture2D::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_occlusion_texture(gl: Arc<glow::Context>) -> Texture2D {
        let data = [
            255u8, 255, // Row 1
            255u8, 255, // Row 2
        ];

        let texture_ir = IRTexture2D {
            data: data.to_vec(),
            width: 2,
            height: 2,
            pixel_format: IRPixelFormat::R8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
        };

        Texture2D::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }
}
