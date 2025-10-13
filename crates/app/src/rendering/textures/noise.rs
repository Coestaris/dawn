use dawn_assets::ir::texture2d::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::texture::Texture2D;
use std::sync::Arc;

pub fn white_noise_rgf32(gl: Arc<glow::Context>, width: usize, height: usize) -> Texture2D {
    let texture = Texture2D::new2d(gl.clone()).unwrap();
    Texture2D::bind(&gl, &texture, 0);
    texture.set_min_filter(IRTextureFilter::Nearest).unwrap();
    texture.set_mag_filter(IRTextureFilter::Nearest).unwrap();
    texture.set_wrap_s(IRTextureWrap::Repeat).unwrap();
    texture.set_wrap_t(IRTextureWrap::Repeat).unwrap();

    let mut data = vec![0f32; width * height * 2];
    for i in 0..(width * height) {
        let x: f32 = rand::random::<f32>();
        let y: f32 = rand::random::<f32>();

        data[i * 2 + 0] = x;
        data[i * 2 + 1] = y;
    }

    texture
        .feed_2d(
            0,
            width,
            height,
            false,
            IRPixelFormat::RG32F,
            Some(data.as_slice()),
        )
        .unwrap();

    Texture2D::unbind(&gl, 0);
    texture
}

/// Generate a white noise texture with f32 values in the range [0.0, 1.0]
pub fn white_noise_f32(gl: Arc<glow::Context>, width: usize, height: usize) -> Texture2D {
    let texture = Texture2D::new2d(gl.clone()).unwrap();
    Texture2D::bind(&gl, &texture, 0);
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

    Texture2D::unbind(&gl, 0);
    texture
}
