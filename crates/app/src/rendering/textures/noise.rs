use dawn_assets::ir::texture::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use std::sync::Arc;

/// Generate a white noise texture with f32 values in the range [0.0, 1.0]
pub fn white_noise_f32(gl: Arc<glow::Context>, width: usize, height: usize) -> Texture {
    let texture = Texture::new2d(gl.clone()).unwrap();
    Texture::bind(&gl, TextureBind::Texture2D, &texture, 0);
    texture.set_min_filter(IRTextureFilter::Nearest).unwrap();
    texture.set_mag_filter(IRTextureFilter::Nearest).unwrap();
    texture.set_wrap_s(IRTextureWrap::Repeat).unwrap();
    texture.set_wrap_t(IRTextureWrap::Repeat).unwrap();

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
