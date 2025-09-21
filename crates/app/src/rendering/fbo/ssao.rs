use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use std::sync::Arc;

// Used for both SSAORaw and SSAOBlurPass
pub struct SSAOTarget {
    pub fbo: Framebuffer,

    // Output texture: R32F
    pub texture: GTexture,
}

impl SSAOTarget {
    pub(crate) fn resize(&self, new_size: UVec2) {
        self.texture.resize(new_size);
    }

    pub fn new(gl: Arc<glow::Context>, size: UVec2) -> Self {
        let target = SSAOTarget {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            texture: GTexture::new(
                gl.clone(),
                IRPixelFormat::R32F,
                FramebufferAttachment::Color0,
            ),
        };

        target.texture.resize(size);

        // Attach texture to the framebuffer
        target.texture.attach(&target.fbo);

        Framebuffer::bind(&gl, &target.fbo);
        target.fbo.draw_buffers(&[target.texture.attachment]);
        assert_eq!(target.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        target
    }
}
