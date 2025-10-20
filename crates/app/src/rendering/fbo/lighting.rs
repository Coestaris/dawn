use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture2d::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use std::rc::Rc;
use std::sync::Arc;

pub struct LightingTarget {
    pub fbo: Framebuffer,

    // Output texture. RGBA8
    // Shared with the transparent target
    pub texture: Rc<GTexture>,
}

pub struct TransparentTarget {
    pub fbo: Framebuffer,

    // Output texture. RGBA8
    // Shared with the lighting target
    pub texture: Rc<GTexture>,
    // Shared depth buffer
    pub depth: Rc<GTexture>,
}

impl LightingTarget {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing RgbBuffer to {:?}", new_size);
        self.texture.resize(new_size);
    }

    pub fn allocate_target(gl: Arc<glow::Context>, initial: UVec2) -> Rc<GTexture> {
        let texture = Rc::new(
            GTexture::new(
                gl.clone(),
                IRPixelFormat::RGB8,
                FramebufferAttachment::Color0,
            )
            .unwrap(),
        );
        texture.resize(initial);
        texture
    }

    pub fn new(gl: Arc<glow::Context>, target: Rc<GTexture>) -> anyhow::Result<Self> {
        let buffer = LightingTarget {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            texture: target,
        };

        // Attach texture to the framebuffer
        buffer.texture.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[buffer.texture.attachment]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
impl TransparentTarget {
    pub fn new(
        gl: Arc<glow::Context>,
        target: Rc<GTexture>,
        depth: Rc<GTexture>,
    ) -> anyhow::Result<Self> {
        let buffer = TransparentTarget {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            texture: target,
            depth,
        };

        // Attach texture to the framebuffer
        buffer.texture.attach(&buffer.fbo);
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        buffer.fbo.draw_buffers(&[buffer.texture.attachment]);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
