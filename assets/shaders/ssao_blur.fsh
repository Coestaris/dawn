#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

out float out_ssao_blur;
in vec2 tex_coord;

// RGB16F view-space position
uniform sampler2D in_position;
// R16F
uniform sampler2D in_ssao_raw;
// RGBA8 (R - roughness, G - occlusion, BA - octo encoded view-space normal)
uniform sampler2D in_rough_occlusion_normal;

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
    vec2 e = texture(in_rough_occlusion_normal, uv).zw;
    return decode_oct(e);
}

void main()
{
    if (in_ssao_enabled != 1) {
        out_ssao_blur = 1.0;
        return;
    }

    vec2 uv = tex_coord;
    vec2 texel = vec2(1.0) / vec2(in_viewport);

    vec3 N = normal(uv);
    float Z = texture(in_position, uv).z;

    float sum = 0.0;
    float wsum = 0.0;

    int R = int(in_radius);
    for (int i = -R; i <= R; i++) {
        for (int j = -R; j <= R; j++) {
            if (i == 0 && j == 0) continue;

            vec2 vector = vec2(i, j);
            vec2 uvn = uv + vector * texel;
            float ao = texture(in_ssao_raw, uvn).r;

            float Zi = texture(in_position, uvn).z;
            vec3 Ni = normal(uvn);

            float w_spatial = gauss(length(vector), in_sigma_spatial);
            float w_depth   = gauss(abs(Zi - Z), in_sigma_depth);
            float w_normal  = pow(max(dot(N, Ni), 0.0), in_sigma_normal);

            float w = w_spatial * w_depth * w_normal;
            sum += ao * w;
            wsum += w;
        }
    }

    if (wsum > 0.0) {
        sum /= wsum;
        out_ssao_blur = sum;
    } else {
        // Fallback if no weights were accumulated
        out_ssao_blur = texture(in_ssao_raw, uv).r;
    }
}