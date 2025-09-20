use dawn_graphics::gl::raii::ubo::UBO;
use glam::Vec4;
use std::sync::Arc;

#[repr(C)]
#[derive(Clone, Copy)]
struct SSAORawParametersUBOPayload {
    kernel_size: u32, // up to 64
    radius: f32,
    bias: f32,
    intensity: f32,
    power: f32,
    _padding: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SSAORawKernelUBOPayload {
    samples: [[f32; 4]; 64],
}

pub struct SSAORawParametersUBO {
    gl: Arc<glow::Context>,
    pub ubo: UBO,
    pub payload: SSAORawParametersUBOPayload,
    pub binding: usize,
}

pub struct SSAORawKernelUBO {
    gl: Arc<glow::Context>,
    pub ubo: UBO,
    pub payload: SSAORawKernelUBOPayload,
    pub binding: usize,
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
        }
    }

    pub fn set_samples<const N: usize>(&mut self) {
        assert!(N <= 64);

        use rand::Rng;
        let mut rng = rand::thread_rng();

        for i in 0..N {
            let scale = (i as f32) / (N as f32);
            let scale = 0.1 + 0.9 * scale * scale;
            let sample = Vec4::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(0.0..1.0),
                0.0,
            )
            .normalize()
                * rng.gen_range(0.0..1.0)
                * scale;
            self.payload.samples[i] = sample.to_array();
        }
    }

    pub fn upload(&self) {
        UBO::bind(&self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const SSAORawKernelUBOPayload as *const u8,
                size_of::<SSAORawKernelUBOPayload>(),
            )
        });
        UBO::unbind(&self.gl);
    }
}

impl SSAORawParametersUBO {
    pub(crate) fn new(gl: Arc<glow::Context>, binding: usize) -> Self {
        let ubo = UBO::new(
            gl.clone(),
            Some(std::mem::size_of::<SSAORawParametersUBOPayload>()),
        )
        .unwrap();
        UBO::bind(&gl, &ubo);
        ubo.bind_base(binding as u32);
        UBO::unbind(&gl);

        Self {
            gl,
            ubo,
            payload: SSAORawParametersUBOPayload {
                kernel_size: 16,
                radius: 0.5,
                bias: 0.025,
                intensity: 1.0,
                power: 1.0,
                _padding: [0; 3],
            },
            binding,
        }
    }

    pub fn set_kernel_size(&mut self, size: u32) {
        assert!(size <= 64);
        self.payload.kernel_size = size;
    }

    pub fn set_radius(&mut self, radius: f32) {
        self.payload.radius = radius;
    }

    pub fn set_bias(&mut self, bias: f32) {
        self.payload.bias = bias;
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.payload.intensity = intensity;
    }

    pub fn set_power(&mut self, power: f32) {
        self.payload.power = power;
    }

    pub fn upload(&self) {
        UBO::bind(&self.gl, &self.ubo);
        self.ubo.feed(unsafe {
            std::slice::from_raw_parts(
                &self.payload as *const SSAORawParametersUBOPayload as *const u8,
                size_of::<SSAORawParametersUBOPayload>(),
            )
        });
        UBO::unbind(&self.gl);
    }
}
