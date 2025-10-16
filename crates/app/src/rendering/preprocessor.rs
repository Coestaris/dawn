use crate::rendering::config;
use glam::{Vec3, Vec4};
use std::collections::HashMap;

fn vec3(v: Vec3) -> String {
    format!("vec3({}, {}, {})", v.x, v.y, v.z)
}

fn vec4(v: Vec4) -> String {
    format!("vec4({}, {}, {}, {})", v.x, v.y, v.z, v.w)
}

fn f32(v: f32) -> String {
    format!("{}", v)
}

fn i32(v: i32) -> String {
    format!("{}", v)
}

fn vec_vec4(v: Vec<Vec4>) -> String {
    let mut s = "vec4[](".to_string();
    for (i, v) in v.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&vec4(*v));
    }
    s.push_str(")");
    s
}

fn vec_f32(v: Vec<f32>) -> String {
    let mut s = "float[](".to_string();
    for (i, v) in v.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&f32(*v));
    }
    s.push_str(")");
    s
}

#[rustfmt::skip]
fn insert_defines(defines: &mut HashMap<String, String>, config: &config::config_static::RenderingConfig) {
    macro_rules! insert_define {
        ($name:expr, $t:expr, $v:expr) => {
            defines.insert($name.to_string(), $t($v));
        };
    }


    #[cfg(feature = "devtools")]
    insert_define!("ENABLE_DEVTOOLS", i32, 1);
    #[cfg(not(feature = "devtools"))]
    insert_define!("ENABLE_DEVTOOLS", i32, 0);

    insert_define!("DEF_DIFFUSE_SCALE", f32, config.get_diffuse_scale());
    insert_define!("DEF_SPECULAR_SCALE", f32, config.get_specular_scale());
    insert_define!("DEF_SSAO_ENABLED", i32, config.get_is_ssao_enabled() as i32);

    insert_define!("DEF_SSAO_RAW_KERNEL_SIZE", i32, config.get_ssao_raw_kernel_size() as i32);
    insert_define!("DEF_SSAO_RAW_RADIUS", f32, config.get_ssao_raw_radius());
    insert_define!("DEF_SSAO_RAW_BIAS", f32, config.get_ssao_raw_bias());
    insert_define!("DEF_SSAO_RAW_INTENSITY", f32, config.get_ssao_raw_intensity());
    insert_define!("DEF_SSAO_RAW_POWER", f32, config.get_ssao_raw_power());
    insert_define!("DEF_SSAO_RAW_KERNEL", vec_vec4, config.get_ssao_raw_kernel());

    insert_define!("DEF_SSAO_BLUR_TAP_COUNT", i32, config.get_ssao_blur_taps_count() as i32);
    insert_define!("DEF_SSAO_BLUR_SIGMA_NORMAL", f32, config.get_ssao_blur_sigma_depth());
    insert_define!("DEF_SSAO_BLUR_TAP_WEIGHT", vec_f32, config.get_ssao_blur_tap_weight());
    insert_define!("DEF_SSAO_BLUR_TAP_OFFSET", vec_f32, config.get_ssao_blur_tap_offset());
}

pub fn shader_defines() -> HashMap<String, String> {
    let mut defines = HashMap::new();
    let config = config::config_static::RenderingConfig::new();
    insert_defines(&mut defines, &config);
    defines
}
