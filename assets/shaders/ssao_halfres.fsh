#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/depth.glsl"
#include "inc/normal.glsl"

// R16F. linear depth
layout (location = 0) out float out_halres_depth;
// RG8 - octo encoded normal, view space
layout (location = 1) out vec2 out_halres_normal;
// RGBA8. RGB - albedo, A - roughness
layout (location = 2) out vec4 out_halres_albedo_rough;

uniform sampler2D in_depth;
// RGBA8. RGB - albedo, A - metallic
uniform sampler2D in_albedo_metallic;
// RGBA8 (R - roughness, G - occlusion, BA - octo encoded view-space normal)
uniform sampler2D in_rough_occlusion_normal;

void main() {
    ivec2 full_size = textureSize(in_depth, 0);
    ivec2 half_size = full_size / 2;

    vec2 full_px = 1.0 / vec2(full_size);

    // Average 4 pixels
    vec2 uv0 = ((gl_FragCoord.xy * 2.0) + vec2(0.5, 0.5)) * full_px;
    vec2 uv1 = uv0 + vec2(full_px.x, 0.0);
    vec2 uv2 = uv0 + vec2(0.0, full_px.y);
    vec2 uv3 = uv0 + full_px;

    float d0 = texture(in_depth, uv0).r;
    float d1 = texture(in_depth, uv1).r;
    float d2 = texture(in_depth, uv2).r;
    float d3 = texture(in_depth, uv3).r;

    float d_min = d0;
    vec2 uv_min = uv0;
    if (d1 < d_min) {
        d_min = d1;
        uv_min = uv1;
    }
    if (d2 < d_min) {
        d_min = d2;
        uv_min = uv2;
    }
    if (d3 < d_min) {
        d_min = d3;
        uv_min = uv3;
    }

    vec2 normal_min = texture(in_rough_occlusion_normal, uv_min).zw;
    vec3 albedo_min = texture(in_albedo_metallic, uv_min).rgb;
    float roughness_min = texture(in_rough_occlusion_normal, uv_min).r;

//    out_halres_depth = linearize_depth(d_min, in_clip_planes.x, in_clip_planes.y);
    out_halres_depth = d_min;
    out_halres_normal = normal_min;
    out_halres_albedo_rough = vec4(albedo_min, roughness_min);
}
