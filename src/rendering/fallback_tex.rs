use crate::rendering::event::RenderingEvent;
use dawn_assets::ir::texture::{IRPixelFormat, IRTexture, IRTextureType};
use dawn_graphics::gl::raii::texture::Texture;

pub struct FallbackTextures<'g> {
    pub albedo_texture: Texture<'g>,
    pub normal_texture: Texture<'g>,
    pub metallic_texture: Texture<'g>,
    pub roughness_texture: Texture<'g>,
    pub occlusion_texture: Texture<'g>,
}

impl<'g> FallbackTextures<'g> {
    pub(crate) fn new(gl: &'g glow::Context) -> Self {
        let albedo_texture = Self::create_missing_albedo_texture(gl);
        let normal_texture = Self::create_missing_normal_texture(gl);
        let metallic_texture = Self::create_missing_metallic_texture(gl);
        let roughness_texture = Self::create_missing_roughness_texture(gl);
        let occlusion_texture = Self::create_missing_occlusion_texture(gl);

        FallbackTextures {
            albedo_texture,
            normal_texture,
            metallic_texture,
            roughness_texture,
            occlusion_texture,
        }
    }

    fn create_missing_albedo_texture(gl: &glow::Context) -> Texture<'_> {
        // Create a 2x2 checkerboard pattern (magenta and black)
        let data: [u8; 12] = [
            255, 0, 255, // Magenta
            0, 0, 0, // Black
            0, 0, 0, // Black
            255, 0, 255, // Magenta
        ];

        let texture_ir = IRTexture {
            data: data.to_vec(),
            texture_type: IRTextureType::Texture2D {
                width: 2,
                height: 2,
            },
            pixel_format: IRPixelFormat::RGB8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        };

        Texture::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_normal_texture(gl: &glow::Context) -> Texture {
        let data = [
            128u8, 128, 255, 128, 128, 255, 128, 128, 255, 128, 128, 255, // Row 1
            128u8, 128, 255, 128, 128, 255, 128, 128, 255, 128, 128, 255, // Row 2
        ];

        let texture_ir = IRTexture {
            data: data.to_vec(),
            texture_type: IRTextureType::Texture2D {
                width: 2,
                height: 2,
            },
            pixel_format: IRPixelFormat::RGB8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        };

        Texture::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_metallic_texture(gl: &glow::Context) -> Texture {
        let data = [
            255u8, 255, 255, 255, // Row 1
            255u8, 255, 255, 255, // Row 2
        ];

        let texture_ir = IRTexture {
            data: data.to_vec(),
            texture_type: IRTextureType::Texture2D {
                width: 2,
                height: 2,
            },
            pixel_format: IRPixelFormat::R8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        };

        Texture::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_roughness_texture(gl: &glow::Context) -> Texture {
        let data = [
            255u8, 255, 255, 255, // Row 1
            255u8, 255, 255, 255, // Row 2
        ];

        let texture_ir = IRTexture {
            data: data.to_vec(),
            texture_type: IRTextureType::Texture2D {
                width: 2,
                height: 2,
            },
            pixel_format: IRPixelFormat::R8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        };

        Texture::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }

    fn create_missing_occlusion_texture(gl: &glow::Context) -> Texture {
        let data = [
            255u8, 255, 255, 255, // Row 1
            255u8, 255, 255, 255, // Row 2
        ];

        let texture_ir = IRTexture {
            data: data.to_vec(),
            texture_type: IRTextureType::Texture2D {
                width: 2,
                height: 2,
            },
            pixel_format: IRPixelFormat::R8,
            use_mipmaps: false,
            min_filter: Default::default(),
            mag_filter: Default::default(),
            wrap_s: Default::default(),
            wrap_t: Default::default(),
            wrap_r: Default::default(),
        };

        Texture::from_ir::<RenderingEvent>(gl, texture_ir)
            .expect("Failed to create missing texture")
            .0
    }
}
