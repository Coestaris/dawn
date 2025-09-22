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
    // RGB16F
    pub position: GTexture,
    // RGBA8. RGB - albedo, A - metallic
    pub albedo_metallic: GTexture,
    // RGB16F. View space
    pub normal: GTexture,
    // RGBA8. R - roughness, G - occlusion, B - emissive, A - reserved
    pub pbr: GTexture, // RGBA8
}

impl GBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.albedo_metallic.resize(new_size);
        self.position.resize(new_size);
        self.normal.resize(new_size);
        self.pbr.resize(new_size);
        self.depth.resize(new_size);
    }

    pub fn new(gl: Arc<glow::Context>, initial: UVec2) -> Self {
        let buffer = GBuffer {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            depth: GTexture::new(
                gl.clone(),
                IRPixelFormat::DEPTH24,
                FramebufferAttachment::Depth,
            ),
            position: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGB16F,
                FramebufferAttachment::Color0,
            ),
            albedo_metallic: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGBA8,
                FramebufferAttachment::Color1,
            ),
            normal: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGB16F,
                FramebufferAttachment::Color2,
            ),
            pbr: GTexture::new(
                gl.clone(),
                IRPixelFormat::RGBA8,
                FramebufferAttachment::Color3,
            ),
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.position.attach(&buffer.fbo);
        buffer.albedo_metallic.attach(&buffer.fbo);
        buffer.normal.attach(&buffer.fbo);
        buffer.pbr.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.position.attachment,
            buffer.albedo_metallic.attachment,
            buffer.normal.attachment,
            buffer.pbr.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        buffer
    }
}
