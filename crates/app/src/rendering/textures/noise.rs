use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use std::sync::Arc;

/// Generate a Vec3F32 white noise texture, where each pixel is a random
/// unit vector in tangent space
pub fn white_noise_tangent_space_f32(
    gl: Arc<glow::Context>,
    width: usize,
    height: usize,
) -> Texture {
    let texture = Texture::new2d(gl.clone()).unwrap();
    Texture::bind(&gl, TextureBind::Texture2D, &texture, 0);
    texture.set_min_filter(IRTextureFilter::Linear).unwrap();
    texture.set_mag_filter(IRTextureFilter::Linear).unwrap();
    texture.set_wrap_s(IRTextureWrap::MirroredRepeat).unwrap();
    texture.set_wrap_t(IRTextureWrap::MirroredRepeat).unwrap();

    let mut data = vec![0f32; width * height * 3];
    for i in 0..(width * height) {
        let x: f32 = rand::random::<f32>() * 2.0 - 1.0;
        let y: f32 = rand::random::<f32>() * 2.0 - 1.0;
        let z: f32 = 0.0; // we want the vector to be in tangent space (z=0)

        // TODO: Do I need to normalize this vector?
        data[i * 3 + 0] = x;
        data[i * 3 + 1] = y;
        data[i * 3 + 2] = z;
    }

    texture
        .feed_2d(
            0,
            width,
            height,
            false,
            IRPixelFormat::RGB32F,
            Some(data.as_slice()),
        )
        .unwrap();

    Texture::unbind(&gl, TextureBind::Texture2D, 0);
    texture
}

/// Generate a white noise texture with f32 values in the range [0.0, 1.0]
pub fn white_noise_f32(gl: Arc<glow::Context>, width: usize, height: usize) -> Texture {
    let texture = Texture::new2d(gl.clone()).unwrap();
    Texture::bind(&gl, TextureBind::Texture2D, &texture, 0);
    texture.set_min_filter(IRTextureFilter::Linear).unwrap();
    texture.set_mag_filter(IRTextureFilter::Linear).unwrap();
    texture.set_wrap_s(IRTextureWrap::MirroredRepeat).unwrap();
    texture.set_wrap_t(IRTextureWrap::MirroredRepeat).unwrap();

    let mut data = vec![0f32; width * height * 4];
    for i in 0..(width * height * 4) {
        data[i] = rand::random::<f32>();
    }
    texture
        .feed_2d(
            0,
            width,
            height,
            false,
            IRPixelFormat::R32F,
            Some(data.as_slice()),
        )
        .unwrap();

    Texture::unbind(&gl, TextureBind::Texture2D, 0);
    texture
}
