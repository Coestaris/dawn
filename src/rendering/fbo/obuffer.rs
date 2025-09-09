use crate::rendering::fbo::GTexture;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use dawn_assets::ir::texture::IRPixelFormat;

pub struct OBuffer {
    pub fbo: Framebuffer,

    // Output texture. RGBA8
    pub texture: GTexture,
}

impl OBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing OBuffer to {:?}", new_size);
        self.texture.resize(new_size);
    }

    pub fn new(gl: &'static glow::Context, initial: UVec2) -> Self {
        let buffer= OBuffer {
            fbo: Framebuffer::new(gl).unwrap(),
            texture: GTexture::new(gl, IRPixelFormat::RGBA8, FramebufferAttachment::Color0),
        };
        
        buffer.resize(initial);
        
        // Attach texture to the framebuffer
        buffer.texture.attach(&buffer.fbo);
        
        Framebuffer::bind(gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[buffer.texture.attachment]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(gl);
        
        buffer
    }
}
