use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use dawn_graphics::gl::raii::renderbuffer::{RenderBufferStorage, Renderbuffer};
use dawn_graphics::gl::raii::texture::{Texture, TextureBind};
use glam::UVec2;
use log::info;

pub struct GTexture<'g> {
    gl: &'g glow::Context,
    pub texture: Texture<'g>,
    pub format: IRPixelFormat,
    pub attachment: FramebufferAttachment,
}

pub struct GRenderBuffer<'g> {
    gl: &'g glow::Context,
    pub render_buffer: Renderbuffer<'g>,
    pub format: RenderBufferStorage,
    pub attachment: FramebufferAttachment,
}

impl<'g> GRenderBuffer<'g> {
    fn new(
        gl: &'g glow::Context,
        format: RenderBufferStorage,
        attachment: FramebufferAttachment,
    ) -> Self {
        let render_buffer = Renderbuffer::new(gl).unwrap();
        GRenderBuffer {
            gl,
            render_buffer,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Renderbuffer::bind(self.gl, &self.render_buffer);
        self.render_buffer
            .storage(self.format, new_size.x as usize, new_size.y as usize);
        Renderbuffer::unbind(self.gl);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(self.gl, fbo);
        fbo.attach_renderbuffer(self.attachment, &self.render_buffer);
        Framebuffer::unbind(self.gl);
    }
}

impl<'g> GTexture<'g> {
    fn new(
        gl: &'g glow::Context,
        format: IRPixelFormat,
        attachment: FramebufferAttachment,
    ) -> Self {
        let texture = Texture::new2d(gl).unwrap();
        Texture::bind(gl, TextureBind::Texture2D, &texture, 0);
        texture.generate_mipmap();
        Texture::unbind(gl, TextureBind::Texture2D, 0);

        GTexture {
            gl,
            texture,
            format,
            attachment,
        }
    }

    fn resize(&self, new_size: UVec2) {
        Texture::bind(self.gl, TextureBind::Texture2D, &self.texture, 0);
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
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
    }

    fn attach(&self, fbo: &Framebuffer) {
        Framebuffer::bind(self.gl, fbo);
        Texture::bind(self.gl, TextureBind::Texture2D, &self.texture, 0);
        fbo.attach_texture_2d(self.attachment, &self.texture, 0);
        assert_eq!(fbo.is_complete(), true);
        Texture::unbind(self.gl, TextureBind::Texture2D, 0);
        Framebuffer::unbind(self.gl);
    }
}

pub struct GBuffer<'g> {
    pub fbo: Framebuffer<'g>,
    pub depth: GRenderBuffer<'g>,
    pub position_texture: GTexture<'g>,
    pub normal_texture: GTexture<'g>,
    pub color_texture: GTexture<'g>,
}

impl<'g> GBuffer<'g> {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.position_texture.resize(new_size);
        self.normal_texture.resize(new_size);
        self.color_texture.resize(new_size);
        self.depth.resize(new_size);
    }

    pub fn new(gl: &'g glow::Context, initial: UVec2) -> Self {
        let buffer = GBuffer {
            fbo: Framebuffer::new(gl).unwrap(),
            depth: GRenderBuffer::new(
                gl,
                RenderBufferStorage::DepthComponent24,
                FramebufferAttachment::Depth,
            ),
            position_texture: GTexture::new(
                gl,
                IRPixelFormat::RGBA16F,
                FramebufferAttachment::Color0,
            ),
            normal_texture: GTexture::new(
                gl,
                IRPixelFormat::RGBA16F,
                FramebufferAttachment::Color1,
            ),
            color_texture: GTexture::new(gl, IRPixelFormat::RGBA8, FramebufferAttachment::Color2),
        };

        buffer.resize(initial);

        // Attach textures to the framebuffer
        buffer.position_texture.attach(&buffer.fbo);
        buffer.normal_texture.attach(&buffer.fbo);
        buffer.color_texture.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[
            buffer.position_texture.attachment,
            buffer.normal_texture.attachment,
            buffer.color_texture.attachment,
        ]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(gl);

        buffer
    }
}
