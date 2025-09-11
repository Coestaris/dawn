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

#[cfg(feature = "devtools")]
mod config_impl {
    use crate::rendering::config::{BoundingBoxMode, OutputMode};
    use std::cell::RefCell;
    use std::rc::Rc;

    pub struct RenderingConfigInner {
        pub wireframe: bool,
        pub fxaa_enabled: bool,
        pub output_mode: OutputMode,
        pub bounding_box_mode: BoundingBoxMode,
        pub show_gizmos: bool,
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
                wireframe: false,
                fxaa_enabled: true,
                output_mode: OutputMode::Default,
                bounding_box_mode: BoundingBoxMode::Disabled,
                show_gizmos: false,
            })))
        }

        pub fn get_is_wireframe(&self) -> bool {
            self.0.borrow().wireframe
        }

        pub fn get_is_fxaa_enabled(&self) -> bool {
            self.0.borrow().fxaa_enabled
        }

        pub fn get_output_mode(&self) -> OutputMode {
            self.0.borrow().output_mode
        }

        pub fn get_bounding_box_mode(&self) -> BoundingBoxMode {
            self.0.borrow().bounding_box_mode
        }

        pub fn get_show_gizmos(&self) -> bool {
            self.0.borrow().show_gizmos
        }
    }
}

#[cfg(not(feature = "devtools"))]
mod config_impl {
    use crate::rendering::config::{BoundingBoxMode, OutputMode};

    #[derive(Debug, Clone, Copy)]
    pub struct RenderingConfig;

    impl RenderingConfig {
        pub fn new() -> Self {
            RenderingConfig {}
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
    }
}

pub use config_impl::RenderingConfig;
