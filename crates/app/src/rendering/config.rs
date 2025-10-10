#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Default,
    AlbedoOnly,
    MetallicOnly,
    NormalOnly,
    RoughnessOnly,
    OcclusionOnly,
    DepthOnly,
    Position,
    SSAOOnly,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundingBoxMode {
    Disabled,
    AABB,
    AABBHonorDepth,
    OBB,
    OBBHonorDepth,
}

pub fn generate_ssao_raw_kernel(size: usize) -> Vec<Vec4> {
    use rand::Rng;
    use rand::SeedableRng;

    // Create a seeded random number generator to keep the kernel consistent
    let mut rng = rand::rngs::StdRng::seed_from_u64(0);
    let mut kernel = vec![Vec4::ZERO; size];

    // Vectors of Normal-oriented hemisphere
    for i in 0..size {
        // Make samples at the center more dense
        let scale = (i as f32) / (size as f32);
        let scale = f32::lerp(0.1, 1.0, scale * scale);

        let sample = Vec4::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(0.0..1.0),
            0.0,
        )
        .normalize()
            * rng.gen_range(0.0..1.0)
            * scale;
        kernel[i] = sample;
    }

    kernel
}

/// Returns (tap_weights, tap_offsets)
pub fn generate_ssao_blur_kernel(taps_count: usize, sigma_spatial: f32) -> (Vec<f32>, Vec<f32>) {
    // Taps must be odd
    assert_eq!(taps_count % 2, 1);

    fn gaussian_kernel_1d(r: usize, sigma: f32) -> Vec<f32> {
        let mut g = vec![0.0; r + 1];
        for i in 0..=r {
            let x = i as f32;
            g[i] = (-0.5 * (x * x) / (sigma * sigma)).exp();
        }
        let mut s = g[0];
        for i in 1..=r {
            s += 2.0 * g[i];
        }
        let mut w = vec![0.0; r + 1];
        for i in 0..=r {
            w[i] = g[i] / s;
        }
        w
    }

    let w = gaussian_kernel_1d(taps_count, sigma_spatial);
    let mut taps: Vec<(f32, f32)> = vec![];

    // Central tap
    taps.push((w[0], 0.0));

    // Pairs
    let mut i = 1;
    while i + 1 <= taps_count {
        let w1 = w[i];
        let w2 = w[i + 1];
        let pair = w1 + w2;
        let o = if pair > 0.0 {
            (w2 - w1) / pair + (i as f32)
        } else {
            i as f32
        };
        taps.push((pair, o));
        i += 2;
    }

    let (tap_weights, tap_offsets): (Vec<f32>, Vec<f32>) = taps.into_iter().unzip();
    (tap_weights, tap_offsets)
}

pub(crate) mod config_static {
    use crate::rendering::config::{BoundingBoxMode, OutputMode};
    use glam::Vec4;

    #[derive(Debug, Clone, Copy)]
    pub struct RenderingConfig;

    impl RenderingConfig {
        pub fn new() -> Self {
            RenderingConfig {}
        }

        #[inline(always)]
        pub fn get_is_ssao_enabled(&self) -> bool {
            true
        }

        #[inline(always)]
        pub fn get_is_wireframe(&self) -> bool {
            false
        }

        #[inline(always)]
        pub fn get_is_fxaa_enabled(&self) -> bool {
            true
        }

        #[inline(always)]
        pub fn get_output_mode(&self) -> OutputMode {
            OutputMode::Default
        }

        #[inline(always)]
        pub fn get_bounding_box_mode(&self) -> BoundingBoxMode {
            BoundingBoxMode::Disabled
        }

        #[inline(always)]
        pub fn get_show_gizmos(&self) -> bool {
            false
        }

        #[inline(always)]
        pub fn get_sky_color(&self) -> glam::Vec3 {
            glam::Vec3::new(0.9, 0.95, 1.0)
        }

        #[inline(always)]
        pub fn get_ground_color(&self) -> glam::Vec3 {
            glam::Vec3::new(0.5, 0.45, 0.4)
        }

        #[inline(always)]
        pub fn get_diffuse_scale(&self) -> f32 {
            1.0
        }

        #[inline(always)]
        pub fn get_specular_scale(&self) -> f32 {
            0.2
        }

        #[inline(always)]
        pub fn get_force_no_tangents(&self) -> bool {
            false
        }

        #[inline(always)]
        pub fn get_ssao_raw_kernel_size(&self) -> u32 {
            24
        }

        #[inline(always)]
        pub fn get_ssao_raw_kernel(&self) -> Vec<Vec4> {
            static KERNEL: once_cell::sync::Lazy<Vec<Vec4>> = once_cell::sync::Lazy::new(|| {
                super::generate_ssao_raw_kernel(
                    RenderingConfig::new().get_ssao_raw_kernel_size() as usize
                )
            });

            KERNEL.clone()
        }

        #[inline(always)]
        pub fn get_ssao_raw_radius(&self) -> f32 {
            0.5
        }

        #[inline(always)]
        pub fn get_ssao_raw_bias(&self) -> f32 {
            0.08
        }

        #[inline(always)]
        pub fn get_ssao_raw_intensity(&self) -> f32 {
            1.0
        }

        #[inline(always)]
        pub fn get_ssao_raw_power(&self) -> f32 {
            1.0
        }

        #[inline(always)]
        pub fn get_ssao_blur_taps_count(&self) -> u32 {
            5
        }

        #[inline(always)]
        pub fn get_ssao_blur_tap_weight(&self) -> Vec<f32> {
            static WEIGHT: once_cell::sync::Lazy<Vec<f32>> = once_cell::sync::Lazy::new(|| {
                super::generate_ssao_blur_kernel(
                    RenderingConfig::new().get_ssao_blur_taps_count() as usize,
                    RenderingConfig::new().get_ssao_blur_sigma_spatial(),
                )
                .0
            });

            WEIGHT.clone()
        }

        #[inline(always)]
        pub fn get_ssao_blur_tap_offset(&self) -> Vec<f32> {
            static OFFSET: once_cell::sync::Lazy<Vec<f32>> = once_cell::sync::Lazy::new(|| {
                super::generate_ssao_blur_kernel(
                    RenderingConfig::new().get_ssao_blur_taps_count() as usize,
                    RenderingConfig::new().get_ssao_blur_sigma_spatial(),
                )
                .1
            });

            OFFSET.clone()
        }

        #[inline(always)]
        pub fn get_ssao_blur_sigma_spatial(&self) -> f32 {
            8.0
        }

        #[inline(always)]
        pub fn get_ssao_blur_sigma_depth(&self) -> f32 {
            8.0
        }
    }
}

#[cfg(feature = "devtools")]
mod config_impl {
    pub(crate) use crate::rendering::config::{config_static, BoundingBoxMode, OutputMode};
    use glam::Vec3;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub struct GeneralConfig {
        pub wireframe: bool,
        pub fxaa_enabled: bool,
        pub ssao_enabled: bool,
        pub output_mode: OutputMode,
        pub bounding_box_mode: BoundingBoxMode,
        pub show_gizmos: bool,
    }

    impl GeneralConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                wireframe: stat.get_is_wireframe(),
                fxaa_enabled: stat.get_is_fxaa_enabled(),
                ssao_enabled: stat.get_is_ssao_enabled(),
                output_mode: stat.get_output_mode(),
                bounding_box_mode: stat.get_bounding_box_mode(),
                show_gizmos: stat.get_show_gizmos(),
            }
        }
    }

    pub struct LightingConfig {
        pub sky_color: Vec3,
        pub ground_color: Vec3,
        pub diffuse_scale: f32,
        pub specular_scale: f32,
        pub force_no_tangents: bool,
    }

    impl LightingConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                sky_color: stat.get_sky_color(),
                ground_color: stat.get_ground_color(),
                diffuse_scale: stat.get_diffuse_scale(),
                specular_scale: stat.get_specular_scale(),
                force_no_tangents: stat.get_force_no_tangents(),
            }
        }
    }

    pub struct SSAORawConfig {
        pub kernel_size: u32,
        pub radius: f32,
        pub bias: f32,
        pub intensity: f32,
        pub power: f32,
        pub kernel: Vec<glam::Vec4>,
    }

    impl SSAORawConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                kernel_size: stat.get_ssao_raw_kernel_size(),
                radius: stat.get_ssao_raw_radius(),
                bias: stat.get_ssao_raw_bias(),
                intensity: stat.get_ssao_raw_intensity(),
                power: stat.get_ssao_raw_power(),
                kernel: stat.get_ssao_raw_kernel(),
            }
        }
    }

    pub struct SSAOBlurConfig {
        pub taps_count: u32,
        pub tap_weight: Vec<f32>,
        pub tap_offset: Vec<f32>,
        pub sigma_depth: f32,
        pub sigma_spatial: f32,
    }

    impl SSAOBlurConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                taps_count: stat.get_ssao_blur_taps_count(),
                tap_weight: stat.get_ssao_blur_tap_weight(),
                tap_offset: stat.get_ssao_blur_tap_offset(),
                sigma_depth: stat.get_ssao_blur_sigma_depth(),
                sigma_spatial: stat.get_ssao_blur_sigma_spatial(),
            }
        }
    }

    pub struct RenderingConfigInner {
        pub general: GeneralConfig,
        pub lighting: LightingConfig,
        pub ssao_raw: SSAORawConfig,
        pub ssao_blur: SSAOBlurConfig,
    }

    pub struct RenderingConfig(pub(crate) Rc<RefCell<RenderingConfigInner>>);

    impl Clone for RenderingConfig {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }

    impl RenderingConfig {
        pub fn new() -> Self {
            Self(Rc::new(RefCell::new(RenderingConfigInner {
                general: GeneralConfig::new(),
                lighting: LightingConfig::new(),
                ssao_raw: SSAORawConfig::new(),
                ssao_blur: SSAOBlurConfig::new(),
            })))
        }

        pub fn get_is_ssao_enabled(&self) -> bool {
            self.0.borrow().general.ssao_enabled
        }

        pub fn get_is_wireframe(&self) -> bool {
            self.0.borrow().general.wireframe
        }

        pub fn get_is_fxaa_enabled(&self) -> bool {
            self.0.borrow().general.fxaa_enabled
        }

        pub fn get_output_mode(&self) -> OutputMode {
            self.0.borrow().general.output_mode
        }

        pub fn get_bounding_box_mode(&self) -> BoundingBoxMode {
            self.0.borrow().general.bounding_box_mode
        }

        pub fn get_show_gizmos(&self) -> bool {
            self.0.borrow().general.show_gizmos
        }

        pub fn get_sky_color(&self) -> glam::Vec3 {
            self.0.borrow().lighting.sky_color
        }

        pub fn get_ground_color(&self) -> glam::Vec3 {
            self.0.borrow().lighting.ground_color
        }

        pub fn get_diffuse_scale(&self) -> f32 {
            self.0.borrow().lighting.diffuse_scale
        }

        pub fn get_specular_scale(&self) -> f32 {
            self.0.borrow().lighting.specular_scale
        }

        pub fn get_force_no_tangents(&self) -> bool {
            self.0.borrow().lighting.force_no_tangents
        }

        pub fn get_ssao_raw_kernel_size(&self) -> u32 {
            self.0.borrow().ssao_raw.kernel_size
        }

        pub fn get_ssao_raw_kernel(&self) -> Vec<glam::Vec4> {
            self.0.borrow().ssao_raw.kernel.clone()
        }

        pub fn get_ssao_raw_radius(&self) -> f32 {
            self.0.borrow().ssao_raw.radius
        }

        pub fn get_ssao_raw_bias(&self) -> f32 {
            self.0.borrow().ssao_raw.bias
        }

        pub fn get_ssao_raw_intensity(&self) -> f32 {
            self.0.borrow().ssao_raw.intensity
        }

        pub fn get_ssao_raw_power(&self) -> f32 {
            self.0.borrow().ssao_raw.power
        }

        pub fn get_ssao_blur_taps_count(&self) -> u32 {
            self.0.borrow().ssao_blur.taps_count
        }

        pub fn get_ssao_blur_tap_weight(&self) -> Vec<f32> {
            self.0.borrow().ssao_blur.tap_weight.clone()
        }

        pub fn get_ssao_blur_tap_offset(&self) -> Vec<f32> {
            self.0.borrow().ssao_blur.tap_offset.clone()
        }

        pub fn get_ssao_blur_sigma_depth(&self) -> f32 {
            self.0.borrow().ssao_blur.sigma_depth
        }

        pub fn get_ssao_blur_sigma_spatial(&self) -> f32 {
            self.0.borrow().ssao_blur.sigma_spatial
        }
    }
}

#[cfg(not(feature = "devtools"))]
pub type RenderingConfig = config_static::RenderingConfig;

#[cfg(feature = "devtools")]
pub use config_impl::RenderingConfig;
use glam::{FloatExt, Vec4};
