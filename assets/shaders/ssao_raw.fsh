#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

// R8
layout(location = 0) out float out_halfres_ssao_raw;

// R16F. Linear depth
uniform sampler2D in_halfres_depth;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_halfres_normal;

#if ENABLE_DEVTOOLS

uniform int   in_kernel_size; // <= 64
uniform float in_radius;      // view-space radius
uniform float in_bias;        // base bias (e.g. 0.02)
uniform float in_intensity;   // optional multiplier
uniform float in_power;       // e.g. 1.0–2.0
uniform int   in_ssao_enabled;

// hemisphere samples in tangent space, z>=0
layout(std140) uniform ubo_ssao_raw_kernel {
    vec4 in_samples[64];
};

#else

const int   in_kernel_size  = DEF_SSAO_KERNEL_SIZE;
const float in_radius       = DEF_SSAO_RADIUS;
const float in_bias         = DEF_SSAO_BIAS;
const float in_intensity    = DEF_SSAO_INTENSITY;
const float in_power        = DEF_SSAO_POWER;
const int   in_ssao_enabled = DEF_SSAO_ENABLED;
const vec4  in_samples[DEF_SSAO_KERNEL_SIZE] = DEF_SSAO_KERNEL;

#endif

float ihash12(ivec2 p) {
    // integer hash -> [0,1)
    uvec2 k = uvec2(p) * uvec2(1664525u, 1013904223u);
    uint n = k.x ^ k.y ^ (k.x << 13);
    return float(n) * (1.0/4294967296.0);
}

vec2 dir_from_hash(float h) {
    float a = 6.2831853 * h;
    return vec2(cos(a), sin(a));
}

vec2 noise2(vec3 view, vec3 normal) {
    vec3 world = (in_inv_view * vec4(view, 1.0)).xyz;
    vec2 grid = floor(world.xz * 0.5);

    const float NOISE_CELL = 0.5;
    ivec2 gXY = ivec2(floor(world.xy / NOISE_CELL));
    ivec2 gXZ = ivec2(floor(world.xz / NOISE_CELL));
    ivec2 gYZ = ivec2(floor(world.yz / NOISE_CELL));

    vec2 dXY = dir_from_hash(ihash12(gXY));
    vec2 dXZ = dir_from_hash(ihash12(gXZ));
    vec2 dYZ = dir_from_hash(ihash12(gYZ));

    vec3 w = abs(normalize(normal));
    w = max(w, 1e-3);
    w /= (w.x + w.y + w.z);

    vec2 d = normalize(dXY * w.z + dXZ * w.y + dYZ * w.x);
    return d;
}

// Helper to keep samples on-screen
bool in_bounds(vec2 uv) {
    return all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
}

vec3 normal(vec2 uv) {
    vec2 e = texture(in_halfres_normal, uv).rg;
    return decode_oct(e);
}

vec3 pos(vec2 uv) {
    float linear = texture(in_halfres_depth, uv).r;
    float depth = depth_from_linear(linear, in_clip_planes.x, in_clip_planes.y);
    return reconstruct_view_pos(depth, uv, in_inv_proj);
}

void main() {
    if (in_ssao_enabled != 1) {
        out_halfres_ssao_raw = 1.0;
        return;
    }

    // View-space position & normal at this pixel
    vec2 uv = gl_FragCoord.xy / vec2(in_viewport) / 0.5; // half-res
    vec3 P = pos(uv);
    vec3 N = normal(uv);

    // build TBN the same way you had (use noise.xy only, z=0)
    vec2 noise2 = noise2(P, N);
    vec3 randT  = normalize(vec3(noise2, 0.0));
    vec3 T = normalize(randT - N * dot(randT, N));
    vec3 B = normalize(cross(N, T));
    mat3 TBN = mat3(T, B, N);

    float occlusion = 0.0;
    float weight_sum = 0.0;

    const float eps = 1e-4;

    for (int i = 0; i < in_kernel_size; ++i) {
        vec3 s = TBN * in_samples[i].xyz;
        vec3 S = P + s * in_radius;

        vec4 clip = in_projection * vec4(S, 1.0);
        if (clip.w <= 0.0) continue;
        vec2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) continue;

        vec3 sceneP = pos(uv);

        float scene_z = sceneP.z;

        float angle_bias = max(in_bias, in_bias * (1.0 - dot(N, normalize(S - P))));
        float occluder   = (scene_z >= S.z + angle_bias) ? 1.0 : 0.0;

        float dist  = length(sceneP - S);
        float range = 1.0 - clamp(dist / in_radius, 0.0, 1.0) + eps;

        occlusion += occluder * range;
        weight_sum += range;
    }

    float ao = 1.0 - (occlusion / (weight_sum + eps));
    ao = pow(clamp(ao, 0.0, 1.0), in_power) * in_intensity;
    out_halfres_ssao_raw = ao;
}
