pub mod billboard;
pub mod geometry;
pub mod lighting;
pub mod line;
pub mod postprocess;
pub mod ssao_raw;
pub mod ssao_blur;
pub mod ssao_halfres;

pub const LINE_SHADER: &str = "line_shader";
pub const GEOMETRY_SHADER: &str = "geometry_shader";
pub const BILLBOARD_SHADER: &str = "billboard_shader";
pub const LIGHTING_SHADER: &str = "lighting_shader";
pub const POSTPROCESS_SHADER: &str = "postprocess_shader";
pub const SSAO_RAW_SHADER: &str = "ssao_raw_shader";
pub const SSAO_BLUR_SHADER: &str = "ssao_blur_shader";
pub const SSAO_HALFRES_SHADER: &str = "ssao_halfres_shader";
