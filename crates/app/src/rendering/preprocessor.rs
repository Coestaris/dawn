use crate::rendering::config;
use glam::Vec4;
use std::collections::HashMap;

trait GLSLPrimitive {
    fn glsl_type() -> String;
}

trait IntoShaderString {
    fn as_shader_string(&self) -> String;
}

impl IntoShaderString for Vec4 {
    fn as_shader_string(&self) -> String {
        format!("vec4({}, {}, {}, {})", self.x, self.y, self.z, self.w)
    }
}

impl IntoShaderString for f32 {
    fn as_shader_string(&self) -> String {
        format!("{}", self)
    }
}

impl IntoShaderString for i32 {
    fn as_shader_string(&self) -> String {
        format!("{}", self)
    }
}

impl GLSLPrimitive for Vec4 {
    fn glsl_type() -> String {
        "vec4".to_string()
    }
}

impl GLSLPrimitive for f32 {
    fn glsl_type() -> String {
        "float".to_string()
    }
}

impl GLSLPrimitive for i32 {
    fn glsl_type() -> String {
        "int".to_string()
    }
}

impl<T> IntoShaderString for Vec<T>
where
    T: IntoShaderString,
    T: GLSLPrimitive,
{
    fn as_shader_string(&self) -> String {
        let mut s = format!("{}[](", T::glsl_type());

        for (i, v) in self.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&v.as_shader_string());
        }
        s.push_str(")");
        s
    }
}

#[rustfmt::skip]
fn insert_defines(defines: &mut HashMap<String, String>, config: &config::config_static::RenderingConfig) {
    macro_rules! insert_define {
        ($name:expr, $v:expr) => {
            defines.insert($name.to_string(), $v.as_shader_string());
        };
    }


    #[cfg(feature = "devtools")]
    insert_define!("ENABLE_DEVTOOLS", 1i32);
    #[cfg(not(feature = "devtools"))]
    insert_define!("ENABLE_DEVTOOLS", 0i32);

    insert_define!("DEF_DIFFUSE_SCALE", config.get_diffuse_scale());
    insert_define!("DEF_SPECULAR_SCALE", config.get_specular_scale());
    insert_define!("DEF_SSAO_ENABLED", config.get_is_ssao_enabled() as i32);

    insert_define!("DEF_SSAO_RAW_KERNEL_SIZE", config.get_ssao_raw_kernel_size() as i32);
    insert_define!("DEF_SSAO_RAW_RADIUS", config.get_ssao_raw_radius());
    insert_define!("DEF_SSAO_RAW_BIAS", config.get_ssao_raw_bias());
    insert_define!("DEF_SSAO_RAW_INTENSITY", config.get_ssao_raw_intensity());
    insert_define!("DEF_SSAO_RAW_POWER", config.get_ssao_raw_power());
    insert_define!("DEF_SSAO_RAW_KERNEL", config.get_ssao_raw_kernel());

    insert_define!("DEF_SSAO_BLUR_TAP_COUNT", config.get_ssao_blur_taps_count() as i32);
    insert_define!("DEF_SSAO_BLUR_SIGMA_NORMAL", config.get_ssao_blur_sigma_depth());
    insert_define!("DEF_SSAO_BLUR_TAP_WEIGHT", config.get_ssao_blur_tap_weight());
    insert_define!("DEF_SSAO_BLUR_TAP_OFFSET", config.get_ssao_blur_tap_offset());
}

pub fn shader_defines() -> HashMap<String, String> {
    let mut defines = HashMap::new();
    let config = config::config_static::RenderingConfig::new();
    insert_defines(&mut defines, &config);
    defines
}
