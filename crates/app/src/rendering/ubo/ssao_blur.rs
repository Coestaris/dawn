use dawn_graphics::gl::raii::ubo::UBO;
use glam::{FloatExt, Vec4};
use log::info;
use std::sync::Arc;

const MAX_TAPS: usize = 32;

// Forced to do this mess, because UBO for kernel is
// just debug only, and I don't want to overcomplicate
// the shaders with vec4 arrays and stuff.
// So we just pad floats to vec4 manually.
// This is wasteful, but whatever.
#[repr(C)]
#[derive(Clone, Copy)]
struct Std140Float {
    v: f32,
    _pad: [f32; 3],
}

impl Std140Float {
    fn new(v: f32) -> Self {
        Self { v, _pad: [0.0; 3] }
    }
}

#[repr(C)]
#[repr(packed)]
#[derive(Clone, Copy)]
pub struct SSAOBlurKernelUBOPayload {
    weight: [Std140Float; MAX_TAPS],
    offset: [Std140Float; MAX_TAPS],
}

pub struct SSAOBlurKernelUBO {
    gl: Arc<glow::Context>,
    pub ubo: UBO,
    pub payload: SSAOBlurKernelUBOPayload,
    pub binding: usize,
    pub fresh: bool,
}

impl SSAOBlurKernelUBO {
    pub(crate) fn new(gl: Arc<glow::Context>, binding: usize) -> Self {
        let ubo = UBO::new(
            gl.clone(),
            Some(std::mem::size_of::<SSAOBlurKernelUBOPayload>()),
        )
        .unwrap();
        UBO::bind(&gl, &ubo);
        ubo.bind_base(binding as u32);
        UBO::unbind(&gl);

        Self {
            gl,
            ubo,
            payload: SSAOBlurKernelUBOPayload {
                weight: [Std140Float::new(0.0); MAX_TAPS],
                offset: [Std140Float::new(0.0); MAX_TAPS],
            },
            binding,
            fresh: false,
        }
    }

    pub fn set_samples(&mut self, weights: Vec<f32>, offsets: Vec<f32>) {
        assert!(weights.len() <= MAX_TAPS);
        assert!(offsets.len() <= MAX_TAPS);
        for (i, weight) in weights.iter().enumerate() {
            self.payload.weight[i] = Std140Float::new(*weight);
        }
        for (i, offset) in offsets.iter().enumerate() {
            self.payload.offset[i] = Std140Float::new(*offset);
        }
        info!(
            "SSAOBlurKernelUBO: set {} weights and {} offsets",
            weights.len(),
            offsets.len()
        );
        self.fresh = false;
    }

    pub fn upload(&mut self) -> bool {
        if self.fresh {
            return false;
        }

        UBO::bind(&self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const SSAOBlurKernelUBOPayload as *const u8,
                size_of::<SSAOBlurKernelUBOPayload>(),
            )
        });
        UBO::unbind(&self.gl);
        self.fresh = true;
        true
    }
}
