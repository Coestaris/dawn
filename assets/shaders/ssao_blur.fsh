#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

#define MAX_TAPS 32

// R8
layout(location = 0) out float out_halfres_ssao_blur;

// R16F. Linear depth
uniform sampler2D in_halfres_depth;
// R8
uniform sampler2D in_halfres_ssao_raw;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_halfres_normal;

// (1 / width, 0) for horizontal, (0, 1 / height) for vertical
// Used to select direction in separable blur
uniform vec2 in_stride;

#if ENABLE_DEVTOOLS

// Full taps.
uniform int in_tap_count;
// i.e., if tap_count = 9, we have 5 weights/offsets
//          center + 4 pairs of symmetric taps
#define ITERATIONS ((in_tap_count) / 2 + 1)

uniform float in_sigma_depth;
uniform int   in_ssao_enabled;

layout(std140) uniform ubo_ssao_blur_taps {
    float in_tap_weight[MAX_TAPS];
    float in_tap_offset[MAX_TAPS];
};

#else

// Half taps, since we do symmetric taps
#define ITERATIONS (DEF_SSAO_BLUR_TAP_COUNT / 2 + 1)

const float in_sigma_depth            = DEF_SSAO_BLUR_SIGMA_NORMAL;
const float in_tap_weight[ITERATIONS] = DEF_SSAO_BLUR_TAP_WEIGHT;
const float in_tap_offset[ITERATIONS] = DEF_SSAO_BLUR_TAP_OFFSET;
const int   in_ssao_enabled           = DEF_SSAO_ENABLED;

#endif

vec3 normal(vec2 uv) {
    return decode_oct(texture(in_halfres_normal, uv).rg);
}

float depth(vec2 uv) {
    return texture(in_halfres_depth, uv).r;
}

void ssao_tap(vec2 uv, vec3 N, float Z, out float ao, out float w) {
    // Fetch tap data
    float tap_ao = texture(in_halfres_ssao_raw, uv).r;
    vec3 tap_n = normal(uv);
    float tap_z = depth(uv);

    // Normal weight
    float tap_d = max(dot(N, tap_n), 0.0);
    // Raise to the 4th power to make it sharper
    float wn = tap_d * tap_d;
    wn *= wn;

    // Depth weight
    float wd = exp(-abs(Z - tap_z) * in_sigma_depth);

    // Combined weight
    float weight = wn * wd;
    ao = tap_ao;
    w = weight;
}

float ssao(vec2 uv) {
    vec3 N = normal(uv);
    float Z = depth(uv);

    float sum = 0.0;
    float wsum = 0.0;

    // Central tap
    {
        float ao = texture(in_halfres_ssao_raw, uv).r;
        float w = in_tap_weight[0];
        sum += ao * w;
        wsum += w;
    }

    // Other taps (symmetric)
    for (int t = 1; t < ITERATIONS; t++) {
        float offset = in_tap_offset[t];
        float weight = in_tap_weight[t];

        vec2 du = in_stride * offset;
        vec2 uvp = uv + du;
        vec2 uvm = uv - du;

        {
            float ao, w;
            ssao_tap(uvp, N, Z, ao, w);
            w *= weight;
            sum += ao * w;
            wsum += w;
        }

        {
            float ao, w;
            ssao_tap(uvm, N, Z, ao, w);
            w *= weight;
            sum += ao * w;
            wsum += w;
        }
    }

    if (wsum > 0.0) {
        sum /= wsum;
        return sum;
    } else {
        // Fallback if no weights were accumulated
        return texture(in_halfres_ssao_raw, uv).r;
    }
}

void main()
{
    if (in_ssao_enabled != 1) {
        out_halfres_ssao_blur = 1.0;
    } else {
        ivec2 size = textureSize(in_halfres_ssao_raw, 0);
        vec2 uv = (gl_FragCoord.xy + 0.5) / vec2(size);
        out_halfres_ssao_blur = ssao(uv);
    }
}