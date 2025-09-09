use crate::rendering::fbo::{GRenderBuffer, GTexture};
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::renderbuffer::{RenderBufferStorage, Renderbuffer};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use glam::UVec2;
use log::info;

pub struct GBuffer {
    pub fbo: Framebuffer,
    // Depth buffer
    pub depth: GTexture,

    // RGBA8. RGB - albedo, A - metallic
    pub albedo_metalic: GTexture,
    // RG16F. View space, Octa-encoded normal
    pub normal: GTexture,
    // RGBA8. R - roughness, G - occlusion, B - emissive, A - reserved
    pub pbr: GTexture, // RGBA8
}

impl GBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.albedo_metalic.resize(new_size);
        self.normal.resize(new_size);
        self.pbr.resize(new_size);
        self.depth.resize(new_size);
    }

    pub fn new(gl: &'static glow::Context, initial: UVec2) -> Self {
        let buffer = GBuffer {
            fbo: Framebuffer::new(gl).unwrap(),
            depth: GTexture::new(gl, IRPixelFormat::DEPTH24, FramebufferAttachment::Depth),
            albedo_metalic: GTexture::new(gl, IRPixelFormat::RGBA8, FramebufferAttachment::Color0),
            normal: GTexture::new(gl, IRPixelFormat::RG16F, FramebufferAttachment::Color1),
            pbr: GTexture::new(gl, IRPixelFormat::RGBA8, FramebufferAttachment::Color2),
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.albedo_metalic.attach(&buffer.fbo);
        buffer.normal.attach(&buffer.fbo);
        buffer.pbr.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.albedo_metalic.attachment,
            buffer.normal.attachment,
            buffer.pbr.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(gl);

        buffer
    }
}
