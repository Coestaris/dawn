use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture2d::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use std::sync::Arc;

pub struct HalfresBuffer {
    pub fbo: Framebuffer,
    // Depth buffer. R16F, linear
    pub depth: GTexture,
    // RG8 - octo encoded normal, view space
    pub normal: GTexture,
}

impl HalfresBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        let half_size = new_size / 2;
        self.depth.resize(half_size);
        self.normal.resize(half_size);
    }

    pub fn new(gl: Arc<glow::Context>, initial: UVec2) -> anyhow::Result<Self> {
        let buffer = HalfresBuffer {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            depth: GTexture::new(
                gl.clone(),
                IRPixelFormat::R16F,
                FramebufferAttachment::Color0,
            )?,
            normal: GTexture::new(
                gl.clone(),
                IRPixelFormat::RG8_SNORM,
                FramebufferAttachment::Color1,
            )?,
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.depth.attach(&buffer.fbo);
        buffer.normal.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer
            .fbo
            .draw_buffers(&[buffer.depth.attachment, buffer.normal.attachment]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
