#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

// R8
layout(location = 0) out float out_ssao_blur;

// DEPTH24. OpenGL default depth format
uniform sampler2D in_depth;
// R8
uniform sampler2D in_ssao_raw_halfres;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_normal;

#if ENABLE_DEVTOOLS

uniform float in_radius;
uniform float in_sigma_spatial;
uniform float in_sigma_depth;
uniform float in_sigma_normal;
uniform int in_ssao_enabled;

#else

const float in_radius = 3.0;
const float in_sigma_spatial = 2.0;
const float in_sigma_depth   = 0.1;
const float in_sigma_normal  = 16.0;
const int in_ssao_enabled = 1;

#endif

float gauss(float x, float s) {
    return exp(-0.5*(x*x)/(s*s));
}

vec3 normal(vec2 uv) {
    return decode_oct(texture(in_normal, uv).rg);
}

float depth(vec2 uv) {
    return linearize_depth(texture(in_depth, uv).r, in_clip_planes.x, in_clip_planes.y);
}

bool in_bounds(vec2 uv) {
    return all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
}

float ssao(vec2 uv_full, vec2 uv_half, vec2 texel, vec2 texel_half) {
    vec3 N = normal(uv_full);
    float Z = depth(uv_full);

    float sum = 0.0;
    float wsum = 0.0;

    int R = int(in_radius);
    for (int i = -R; i <= R; i++) {
        for (int j = -R; j <= R; j++) {
            vec2 vector = vec2(i, j);
            vec2 uvn_full = uv_full + vector * texel;
            vec2 uvn_half = uv_half + vector * texel_half;

            float ao = texture(in_ssao_raw_halfres, uvn_half).r;
            float Zi = depth(uvn_full);
            vec3 Ni = normal(uvn_full);

            float w_spatial = gauss(pow(length(vector), 2.0), in_sigma_spatial);
            float w_depth   = gauss(abs(Zi - Z), in_sigma_depth);
            float w_normal  = pow(max(dot(N, Ni), 0.0), in_sigma_normal);

            float w = w_spatial * w_depth * w_normal;
            sum += ao * w;
            wsum += w;
        }
    }

    if (wsum > 0.0) {
        sum /= wsum;
        return sum;
    } else {
        // Fallback if no weights were accumulated
        return 0.5; // Neutral AO value
    }
}

void main()
{
    vec2 texel = vec2(1.0) / vec2(in_viewport);
    vec2 texel_half = texel * 2.0;

    vec2 uv_full = (gl_FragCoord.xy + 0.5) * texel;
    vec2 uv_half = (floor(gl_FragCoord.xy * 0.5) + 0.5) * texel_half;

    if (in_ssao_enabled != 1) {
        out_ssao_blur = 1.0;
    } else {
        out_ssao_blur = ssao(uv_full, uv_half, texel, texel_half);
    }
}