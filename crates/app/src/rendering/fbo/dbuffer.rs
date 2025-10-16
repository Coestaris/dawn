use crate::rendering::fbo::GTexture;
use dawn_assets::ir::texture2d::IRPixelFormat;
use dawn_graphics::gl::raii::framebuffer::{Framebuffer, FramebufferAttachment};
use glam::UVec2;
use log::info;
use std::rc::Rc;
use std::sync::Arc;

pub struct DBuffer {
    pub fbo: Framebuffer,
    // Depth24.
    pub depth: Rc<GTexture>,
}

impl DBuffer {
    pub(crate) fn resize(&self, new_size: UVec2) {
        info!("Resizing GBuffer to {:?}", new_size);
        self.depth.resize(new_size);
    }

    pub fn allocate_depth(gl: Arc<glow::Context>, initial: UVec2) -> Rc<GTexture> {
        let depth = Rc::new(
            GTexture::new(
                gl.clone(),
                IRPixelFormat::DEPTH24,
                FramebufferAttachment::Depth,
            )
            .unwrap(),
        );
        depth.resize(initial);
        depth
    }

    pub fn new(
        gl: Arc<glow::Context>, 
        depth: Rc<GTexture>,
    ) -> Result<Self, String> {
        let buffer = DBuffer {
            fbo: Framebuffer::new(gl.clone()).unwrap(),
            depth: depth.clone(),
        };

        // Attach textures to the framebuffer
        buffer.depth.attach(&buffer.fbo);

        Framebuffer::bind(&gl, &buffer.fbo);
        assert_eq!(buffer.fbo.is_complete(), true);
        Framebuffer::unbind(&gl);

        Ok(buffer)
    }
}
