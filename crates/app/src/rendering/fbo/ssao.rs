use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture2d::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use std::sync::Arc;

pub struct SSAOTarget {
    pub fbo: Framebuffer,

    // Output texture: R8
    pub texture: GTexture,
}

impl SSAOTarget {
    pub(crate) fn resize(&self, new_size: UVec2) {
        self.texture.resize(UVec2::new(new_size.x, new_size.y));
    }

    pub fn new(gl: Arc<glow::Context>, size: UVec2) -> anyhow::Result<Self> {
        let target = SSAOTarget {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            texture: GTexture::new(gl.clone(), IRPixelFormat::R8, FramebufferAttachment::Color0)?,
        };

        target.texture.resize(size);

        // Attach texture to the framebuffer
        target.texture.attach(&target.fbo);

        Framebuffer::bind(&gl, &target.fbo);
        target.fbo.draw_buffers(&[target.texture.attachment]);
        assert_eq!(target.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(target)
    }
}

pub struct SSAOHalfresTarget {
    pub fbo: Framebuffer,

    // Output texture: R8
    pub texture: GTexture,
}

impl SSAOHalfresTarget {
    pub(crate) fn resize(&self, new_size: UVec2) {
        let new_size = new_size / 2;
        self.texture.resize(UVec2::new(new_size.x, new_size.y));
    }

    pub fn new(gl: Arc<glow::Context>, size: UVec2) -> anyhow::Result<Self> {
        let target = SSAOHalfresTarget {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            texture: GTexture::new(gl.clone(), IRPixelFormat::R8, FramebufferAttachment::Color0)?,
        };

        target.texture.resize(size);

        // Attach texture to the framebuffer
        target.texture.attach(&target.fbo);

        Framebuffer::bind(&gl, &target.fbo);
        target.fbo.draw_buffers(&[target.texture.attachment]);
        assert_eq!(target.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(target)
    }
}
