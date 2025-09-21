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

pub(crate) mod config_static {
    use crate::rendering::config::{BoundingBoxMode, OutputMode};

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
            true
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
        pub fn get_ssao_kernel_size(&self) -> u32 {
            16
        }

        #[inline(always)]
        pub fn get_ssao_radius(&self) -> f32 {
            0.5
        }

        #[inline(always)]
        pub fn get_ssao_bias(&self) -> f32 {
            0.025
        }

        #[inline(always)]
        pub fn get_ssao_intensity(&self) -> f32 {
            1.0
        }

        #[inline(always)]
        pub fn get_ssao_power(&self) -> f32 {
            1.0
        }

        #[inline(always)]
        pub fn get_ssao_blur_sigma_spatial(&self) -> f32 {
            2.0
        }

        #[inline(always)]
        pub fn get_ssao_blur_sigma_depth(&self) -> f32 {
            0.1
        }

        #[inline(always)]
        pub fn get_ssao_blur_sigma_normal(&self) -> f32 {
            0.1
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
    }

    impl SSAORawConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                kernel_size: stat.get_ssao_kernel_size(),
                radius: stat.get_ssao_radius(),
                bias: stat.get_ssao_bias(),
                intensity: stat.get_ssao_intensity(),
                power: stat.get_ssao_power(),
            }
        }
    }

    pub struct SSAOBlurConfig {
        pub sigma_spatial: f32,
        pub sigma_depth: f32,
        pub sigma_normal: f32,
    }

    impl SSAOBlurConfig {
        pub fn new() -> Self {
            let stat = config_static::RenderingConfig::new();
            Self {
                sigma_spatial: stat.get_ssao_blur_sigma_spatial(),
                sigma_depth: stat.get_ssao_blur_sigma_depth(),
                sigma_normal: stat.get_ssao_blur_sigma_normal(),
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

        pub fn get_ssao_kernel_size(&self) -> u32 {
            self.0.borrow().ssao_raw.kernel_size
        }

        pub fn get_ssao_radius(&self) -> f32 {
            self.0.borrow().ssao_raw.radius
        }

        pub fn get_ssao_bias(&self) -> f32 {
            self.0.borrow().ssao_raw.bias
        }

        pub fn get_ssao_intensity(&self) -> f32 {
            self.0.borrow().ssao_raw.intensity
        }

        pub fn get_ssao_power(&self) -> f32 {
            self.0.borrow().ssao_raw.power
        }

        pub fn get_ssao_blur_sigma_spatial(&self) -> f32 {
            self.0.borrow().ssao_blur.sigma_spatial
        }

        pub fn get_ssao_blur_sigma_depth(&self) -> f32 {
            self.0.borrow().ssao_blur.sigma_depth
        }

        pub fn get_ssao_blur_sigma_normal(&self) -> f32 {
            self.0.borrow().ssao_blur.sigma_normal
        }
    }
}

#[cfg(not(feature = "devtools"))]
pub type RenderingConfig = config_static::RenderingConfig;
#[cfg(feature = "devtools")]
pub use config_impl::{
    GeneralConfig, LightingConfig, RenderingConfig, SSAOBlurConfig, SSAORawConfig,
};
