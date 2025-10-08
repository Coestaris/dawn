use dawn_graphics::gl::raii::ubo::UBO;
use glam::{FloatExt, Vec4};
use log::info;
use std::sync::Arc;

#[repr(C)]
#[repr(packed)]
#[derive(Clone, Copy)]
pub struct SSAORawKernelUBOPayload {
    samples: [[f32; 4]; 64],
}

pub struct SSAORawKernelUBO {
    gl: Arc<glow::Context>,
    pub ubo: UBO,
    pub payload: SSAORawKernelUBOPayload,
    pub binding: usize,
    pub fresh: bool,
}

impl SSAORawKernelUBO {
    pub(crate) fn new(gl: Arc<glow::Context>, binding: usize) -> Self {
        let ubo = UBO::new(
            gl.clone(),
            Some(std::mem::size_of::<SSAORawKernelUBOPayload>()),
        )
        .unwrap();
        UBO::bind(&gl, &ubo);
        ubo.bind_base(binding as u32);
        UBO::unbind(&gl);

        Self {
            gl,
            ubo,
            payload: SSAORawKernelUBOPayload {
                samples: [[0.0; 4]; 64],
            },
            binding,
            fresh: false,
        }
    }

    pub fn set_samples(&mut self, kernel: Vec<Vec4>) {
        assert!(kernel.len() <= 64);
        for (i, sample) in kernel.iter().enumerate() {
            self.payload.samples[i] = sample.to_array();
        }
        info!("SSAORawKernelUBO: set {} samples", kernel.len());
        self.fresh = false;
    }

    pub fn upload(&mut self) -> bool {
        if self.fresh {
            return false;
        }

        UBO::bind(&self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const SSAORawKernelUBOPayload as *const u8,
                size_of::<SSAORawKernelUBOPayload>(),
            )
        });
        UBO::unbind(&self.gl);
        self.fresh = true;
        true
    }
}
