#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

out float out_ssao_blur;
in vec2 tex_coord;

uniform sampler2D in_ssao_raw;
uniform sampler2D in_depth;
uniform sampler2D in_normal;

#if ENABLE_DEVTOOLS

uniform float in_sigma_spatial;
uniform float in_sigma_depth;
uniform float in_sigma_normal;

#else

const float in_sigma_spatial = 2.0;
const float in_sigma_depth   = 0.1;
const float in_sigma_normal  = 16.0;

#endif

float gauss(float x, float s) {
    return exp(-0.5*(x*x)/(s*s));
}

vec3 normal(vec2 uv) {
    vec2 oct_normal = texture(in_normal, tex_coord).rg;
    vec3 normal = decode_oct(oct_normal);
    return normalize(normal);
}

void main()
{
    vec2 uv = tex_coord;
    vec2 texel = in_viewport;

    vec3 N0 = texture(in_normal, uv).xyz * 2.0 - 1.0;
    float d0 = texture(in_depth, uv).r;
    if (d0 >= 1.0) {
        out_ssao_blur = 1.0;
        return;
    }
    float z0 = linearize_depth(d0, in_clip_planes.x, in_clip_planes.y);

    float sum = 0.0;
    float wsum = 0.0;

    const int R = 4;// filter radius
    for (int i = -R; i <= R; i++) {
        for (int j = -R; j <= R; j++) {
            if (i == 0 && j == 0) continue;

            vec2 uvn = uv + vec2(i, j) * texel;
            float ao = texture(in_ssao_raw, uvn).r;

            float di = texture(in_depth, uvn).r;
            float zi;
            if (di >= 1.0) {
                ao = 1.0;
                zi = z0;// ignore depth difference
            } else {
                zi = linearize_depth(di, in_clip_planes.x, in_clip_planes.y);
            }

            vec3 Ni = normal(uvn);

            float w_spatial = gauss(length(vec2(i, j)), in_sigma_spatial);
            float w_depth   = gauss(abs(zi - z0), in_sigma_depth);
            float w_normal  = pow(max(dot(N0, Ni), 0.0), in_sigma_normal);

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