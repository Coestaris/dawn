use dawn_assets::ir::texture2d::{IRPixelFormat, IRTextureFilter, IRTextureWrap};
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::renderbuffer::{RenderBufferStorage, Renderbuffer};
use dawn_graphics::gl::raii::texture::{GLTexture, Texture2D};
use glam::UVec2;
use std::sync::Arc;

pub mod gbuffer;
pub mod halfres;
pub mod obuffer;
pub mod ssao;

#[allow(dead_code)]
pub struct GTexture {
    gl: Arc<glow::Context>,
    pub texture: Texture2D,
    pub format: IRPixelFormat,
    pub attachment: FramebufferAttachment,
}

#[allow(dead_code)]
pub struct GRenderBuffer {
    gl: Arc<glow::Context>,
    pub render_buffer: Renderbuffer,
    pub format: RenderBufferStorage,
    pub attachment: FramebufferAttachment,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
impl GTexture {
    fn new(
        gl: Arc<glow::Context>,
        format: IRPixelFormat,
        attachment: FramebufferAttachment,
    ) -> anyhow::Result<Self> {
        let texture = Texture2D::new(gl.clone())?;
        Texture2D::bind(&gl, &texture, 0);
        texture.set_wrap_s(IRTextureWrap::ClampToEdge)?;
        texture.set_wrap_t(IRTextureWrap::ClampToEdge)?;
        texture.set_min_filter(IRTextureFilter::Nearest)?;
        texture.set_mag_filter(IRTextureFilter::Nearest)?;
        texture.disable_compare_mode()?;
        texture.set_max_level(0)?;
        Texture2D::unbind(&gl, 0);

        Ok(GTexture {
            gl,
            texture,
            format,
            attachment,
        })
    }

    fn resize(&self, new_size: UVec2) {
        Texture2D::bind(&self.gl, &self.texture, 0);
        self.texture
            .feed::<()>(
                0,
                new_size.x as usize,
                new_size.y as usize,
                false,
                self.format,
                None,
            )
            .unwrap();
        self.texture.generate_mipmap();
        Texture2D::unbind(&self.gl, 0);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(&self.gl, fbo);
        Texture2D::bind(&self.gl, &self.texture, 0);
        fbo.attach_texture_2d(self.attachment, &self.texture, 0);
        Texture2D::unbind(&self.gl, 0);
        Framebuffer::unbind(&self.gl);
    }

    pub fn bind2d(&self, index: i32) {
        Texture2D::bind(&self.gl, &self.texture, index as u32);
    }
}
