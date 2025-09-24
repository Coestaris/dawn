use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use std::sync::Arc;

pub struct GBuffer {
    pub fbo: Framebuffer,
    // Depth buffer
    pub depth: GTexture,
    // RGBA8. RGB - albedo, A - metallic
    pub albedo_metallic: GTexture,
    // RGBA8. R - roughness, G - occlusion, BA - octo encoded normal, view space
    pub rough_occlusion_normal: GTexture,
}

impl GBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.albedo_metallic.resize(new_size);
        self.rough_occlusion_normal.resize(new_size);
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
            albedo_metallic: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGBA8,
                FramebufferAttachment::Color1,
            )?,
            rough_occlusion_normal: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGBA8,
                FramebufferAttachment::Color3,
            )?,
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.albedo_metallic.attach(&buffer.fbo);
        buffer.rough_occlusion_normal.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.albedo_metallic.attachment,
            buffer.rough_occlusion_normal.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
