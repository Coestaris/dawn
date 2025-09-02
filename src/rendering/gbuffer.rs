use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::bindings;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::renderbuffer::{RenderBufferStorage, Renderbuffer};
use dawn_graphics::gl::raii::texture::Texture;
use glam::UVec2;
use log::info;

pub struct GTexture {
    pub texture: Texture,
    pub format: IRPixelFormat,
    pub attachment: FramebufferAttachment,
}

pub struct GRenderBuffer {
    pub render_buffer: Renderbuffer,
    pub format: RenderBufferStorage,
    pub attachment: FramebufferAttachment,
}

impl GRenderBuffer {
    fn new(format: RenderBufferStorage, attachment: FramebufferAttachment) -> Self {
        let render_buffer = Renderbuffer::new().unwrap();
        GRenderBuffer {
            render_buffer,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Renderbuffer::bind(&self.render_buffer);
        self.render_buffer
            .storage(self.format, new_size.x as usize, new_size.y as usize);
        Renderbuffer::unbind();
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(fbo);
        fbo.attach_renderbuffer(self.attachment, &self.render_buffer);
        Framebuffer::unbind();
    }
}

impl GTexture {
    fn new(format: IRPixelFormat, attachment: FramebufferAttachment) -> Self {
        let texture = Texture::new2d().unwrap();
        Texture::bind(bindings::TEXTURE_2D, &texture, 0);
        texture.generate_mipmap();
        Texture::unbind(bindings::TEXTURE_2D, 0);

        GTexture {
            texture,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Texture::bind(bindings::TEXTURE_2D, &self.texture, 0);
        self.texture
            .texture_image_2d(
                0,
                new_size.x as usize,
                new_size.y as usize,
                false,
                self.format,
                None,
            )
            .unwrap();
        self.texture.generate_mipmap();
        Texture::unbind(bindings::TEXTURE_2D, 0);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(fbo);
        Texture::bind(bindings::TEXTURE_2D, &self.texture, 0);
        fbo.attach_texture_2d(self.attachment, &self.texture, 0);
        assert_eq!(fbo.is_complete(), true);
        Texture::unbind(bindings::TEXTURE_2D, 0);
        Framebuffer::unbind();
    }
}

pub struct GBuffer {
    pub fbo: Framebuffer,
    pub depth: GRenderBuffer,
    pub position_texture: GTexture,
    pub normal_texture: GTexture,
    pub color_texture: GTexture,
}

impl GBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.position_texture.resize(new_size);
        self.normal_texture.resize(new_size);
        self.color_texture.resize(new_size);
        self.depth.resize(new_size);
    }

    pub fn new(initial: UVec2) -> Self {
        let buffer = GBuffer {
            fbo: Framebuffer::new().unwrap(),
            depth: GRenderBuffer::new(
                RenderBufferStorage::DepthComponent24,
                FramebufferAttachment::Depth,
            ),
            position_texture: GTexture::new(IRPixelFormat::RGBA16F, FramebufferAttachment::Color0),
            normal_texture: GTexture::new(IRPixelFormat::RGBA16F, FramebufferAttachment::Color1),
            color_texture: GTexture::new(IRPixelFormat::RGBA8, FramebufferAttachment::Color2),
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.position_texture.attach(&buffer.fbo);
        buffer.normal_texture.attach(&buffer.fbo);
        buffer.color_texture.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.position_texture.attachment,
            buffer.normal_texture.attachment,
            buffer.color_texture.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind();

        buffer
    }
}
