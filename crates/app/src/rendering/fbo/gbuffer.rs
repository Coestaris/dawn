use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use std::sync::Arc;

pub struct GBuffer {
    pub fbo: Framebuffer,

    // Depth24.
    pub depth: GTexture,
    // RGB8.
    pub albedo: GTexture,
    // RGB8. R - occlusion, G - roughness, B - metallic
    pub orm: GTexture,
    // RG8_SNORM. Octo encoded normal, view space
    pub normal: GTexture,
}

impl GBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.albedo.resize(new_size);
        self.orm.resize(new_size);
        self.normal.resize(new_size);
        self.depth.resize(new_size);
    }

    pub fn new(gl: Arc<glow::Context>, initial: UVec2) -> anyhow::Result<Self> {
        let buffer = GBuffer {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            depth: GTexture::new(
                gl.clone(),
                IRPixelFormat::DEPTH24,
                FramebufferAttachment::Depth,
            )?,
            albedo: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGB8,
                FramebufferAttachment::Color0,
            )?,
            orm: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGB8,
                FramebufferAttachment::Color1,
            )?,
            normal: GTexture::new(
                gl.clone(),
                IRPixelFormat::RG8_SNORM,
                FramebufferAttachment::Color2,
            )?,
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.albedo.attach(&buffer.fbo);
        buffer.orm.attach(&buffer.fbo);
        buffer.normal.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.albedo.attachment,
            buffer.orm.attachment,
            buffer.normal.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
