#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/depth.glsl"
#include "inc/normal.glsl"

// R16F. Linear depth
layout (location = 0) out float out_halres_depth;
// RG8_SNORM. Octo encoded normal, view space
layout (location = 1) out vec2 out_halres_normal;

// DEPTH24. OpenGL default depth format
uniform sampler2D in_depth;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_normal;

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

    out_halres_depth = linearize_depth(d_min, in_clip_planes.x, in_clip_planes.y);
    out_halres_normal = texture(in_normal, uv_min).rg;;
}
