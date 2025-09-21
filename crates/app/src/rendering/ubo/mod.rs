pub mod camera;
pub mod packed_light;
pub mod ssao_raw;

pub const CAMERA_UBO_BINDING: usize = 0;
pub const SSAO_RAW_PARAMS_UBO_BINDING: usize = 1;
pub const SSAO_RAW_KERNEL_UBO_BINDING: usize = 2;
