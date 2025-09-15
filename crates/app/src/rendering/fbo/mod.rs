use std::sync::Arc;
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::renderbuffer::{RenderBufferStorage, Renderbuffer};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use glam::UVec2;

pub mod gbuffer;
pub mod obuffer;

pub struct GTexture {
    gl: Arc<glow::Context>,
    pub texture: Texture,
    pub format: IRPixelFormat,
    pub attachment: FramebufferAttachment,
}

pub struct GRenderBuffer {
    gl: Arc<glow::Context>,
    pub render_buffer: Renderbuffer,
    pub format: RenderBufferStorage,
    pub attachment: FramebufferAttachment,
}

impl GRenderBuffer {
    fn new(
        gl: Arc<glow::Context>,
        format: RenderBufferStorage,
        attachment: FramebufferAttachment,
    ) -> Self {
        let render_buffer = Renderbuffer::new(gl.clone()).unwrap();
        GRenderBuffer {
            gl,
            render_buffer,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Renderbuffer::bind(&self.gl, &self.render_buffer);
        self.render_buffer
            .storage(self.format, new_size.x as usize, new_size.y as usize);
        Renderbuffer::unbind(&self.gl);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(&self.gl, fbo);
        fbo.attach_renderbuffer(self.attachment, &self.render_buffer);
        Framebuffer::unbind(&self.gl);
    }
}

impl GTexture {
    fn new(
        gl: Arc<glow::Context>,
        format: IRPixelFormat,
        attachment: FramebufferAttachment,
    ) -> Self {
        let texture = Texture::new2d(gl.clone()).unwrap();
        Texture::bind(&gl, TextureBind::Texture2D, &texture, 0);
        texture.generate_mipmap();
        Texture::unbind(&gl, TextureBind::Texture2D, 0);

        GTexture {
            gl,
            texture,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Texture::bind(&self.gl, TextureBind::Texture2D, &self.texture, 0);
        self.texture
            .feed_2d(
                0,
                new_size.x as usize,
                new_size.y as usize,
                false,
                self.format,
                None,
            )
            .unwrap();
        self.texture.generate_mipmap();
        Texture::unbind(&self.gl, TextureBind::Texture2D, 0);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(&self.gl, fbo);
        Texture::bind(&self.gl, TextureBind::Texture2D, &self.texture, 0);
        fbo.attach_texture_2d(self.attachment, &self.texture, 0);
        assert_eq!(fbo.is_complete(), true);
        Texture::unbind(&self.gl, TextureBind::Texture2D, 0);
        Framebuffer::unbind(&self.gl);
    }
}
