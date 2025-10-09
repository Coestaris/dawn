#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

#define MAX_TAPS 9

// R8
layout(location = 0) out float out_halfres_ssao_blur;

// R16F. Linear depth
uniform sampler2D in_halfres_depth;
// R8
uniform sampler2D in_halfres_ssao_raw;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_halfres_normal;

// (1 / width, 0) for vertical and (0, 1 / height) for horizontal
// Used to select direction in separable blur
uniform vec2 stride;

#if ENABLE_DEVTOOLS

uniform int   in_tap_count;
uniform float in_sigma_depth;
uniform int   in_ssao_enabled;

layout(std140) uniform ubo_ssao_blur_taps {
    float in_tap_weight[MAX_TAPS];
    float in_tap_offset[MAX_TAPS];
};

#else

const int   in_tap_count                           = DEF_SSAO_BLUR_TAP_COUNT;
const float in_sigma_depth                         = DEF_SSAO_BLUR_SIGMA_NORMAL;
const float in_tap_weight[DEF_SSAO_BLUR_TAP_COUNT] = DEF_SSAO_BLUR_TAP_WEIGHT;
const float in_tap_offset[DEF_SSAO_BLUR_TAP_COUNT] = DEF_SSAO_BLUR_TAP_OFFSET;
const int   in_ssao_enabled                        = DEF_SSAO_ENABLED;

#endif

vec3 normal(vec2 uv) {
    return decode_oct(texture(in_halfres_normal, uv).rg);
}

float depth(vec2 uv) {
    return texture(in_halfres_depth, uv).r;
}

bool in_bounds(vec2 uv) {
    return all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
}

float ssao(vec2 uv) {
    vec3 N = normal(uv);
    float Z = depth(uv);

    float sum = 0.0;
    float wsum = 0.0;

    // Central tap
    {
        float ao = texture(in_ssao_raw_halfres, uv).r;
        float w = in_tap_weight[0];
        sum += ao * w;
        wsum += w;
    }

    // Other taps
    for (int t = 1; t < in_tap_count; t++) {
        float offset = in_tap_offset[t];

        vec2 du = stride * offset;
        vec2 uvp = uv + du;
        vec2 uvm = uv - du;

        float aop = texture(in_ssao_raw_halfres, uvp).r;
        float aom = texture(in_ssao_raw_halfres, uvm).r;

        vec3 Np = normal(uvp);
        vec3 Nm = normal(uvm);
        float Zp = depth(uvp);
        float Zm = depth(uvm);

        // Weight by normal similarity
        float dp = max(dot(N, Np), 0.0);
        float dm = max(dot(N, Nm), 0.0);

        // Raise to the 4th power to make it sharper
        float wnp = dp * dp;
        float wnm = dm * dm;
        wnp *= wnp;
        wnm *= wnm;

        float wdp = exp(-abs(Z - Zp) * in_sigma_depth);
        float wdm = exp(-abs(Z - Zm) * in_sigma_depth);

        float w = in_tap_weight[t];
        float wp = w * wnp * wdp;
        float wm = w * wnm * wdm;

        sum += aop * wp + aom * wm;
        wsum += wp + wm;
    }

    if (wsum > 0.0) {
        sum /= wsum;
        return sum;
    } else {
        // Fallback if no weights were accumulated
        return texture(in_ssao_raw_halfres, uv).r;
    }
}

void main()
{
    if (in_ssao_enabled != 1) {
        out_halfres_ssao_blur = 1.0;
    } else {
        out_halfres_ssao_blur = ssao(gl_FragCoord.xy / vec2(in_viewport));
    }
}