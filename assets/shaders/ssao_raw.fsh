#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

uniform sampler2D in_depth;
// RGBA8 (R - roughness, G - occlusion, BA - octo encoded view-space normal)
uniform sampler2D in_rough_occlusion_normal;

layout(std140) uniform ubo_ssao_raw_params {
    float in_kernel_size; // <= 64
    float in_radius;      // view-space radius
    float in_bias;        // base bias (e.g. 0.02)
    float in_intensity;   // optional multiplier
    float in_power;       // e.g. 1.0–2.0
    int in_ssao_enabled;
    vec2 _padding;
};

layout(std140) uniform ubo_ssao_raw_kernel {
    vec4 in_samples[64]; // hemisphere samples in tangent space, z>=0
};

out float out_ssao_raw;

in vec2 tex_coord;

float hash12(vec2 p){
    vec3 p3 = fract(vec3(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y)*p3.z);
}

vec2 noise2() {
    float angle = 6.28318 * hash12(floor(gl_FragCoord.xy));
    float s = sin(angle);
    float c = cos(angle);
    return vec2(c, s);
}

// Helper to keep samples on-screen
bool in_bounds(vec2 uv) {
    return all(greaterThanEqual(uv, vec2(0.0))) && all(lessThanEqual(uv, vec2(1.0)));
}

vec3 normal(vec2 uv) {
    vec2 e = texture(in_rough_occlusion_normal, uv).zw;
    return decode_oct(e);
}

float depth(vec2 uv) {
    return linearize_depth(texture(in_depth, uv).r, in_clip_planes.x, in_clip_planes.y);
}

vec3 pos(vec2 uv) {
    return reconstruct_view_pos(texture(in_depth, uv).r, uv, in_inv_proj);
}

void main() {
    if (in_ssao_enabled != 1) {
        out_ssao_raw = 1.0;
        return;
    }

    // View-space position & normal at this pixel
    vec3 P = pos(tex_coord);
    vec3 N = normal(tex_coord);

    // build TBN the same way you had (use noise.xy only, z=0)
    vec2 noise2 = noise2();
    vec3 randT  = normalize(vec3(noise2, 0.0));
    vec3 T = normalize(randT - N * dot(randT, N));
    vec3 B = normalize(cross(N, T));
    mat3 TBN = mat3(T, B, N);

    float occlusion = 0.0;
    const float eps = 1e-4;

    int kernel_size = int(in_kernel_size);
    for (int i = 0; i < kernel_size; ++i) {
        vec3 s = TBN * in_samples[i].xyz;
        vec3 S = P + s * in_radius;

        vec4 clip = in_projection * vec4(S, 1.0);
        if (clip.w <= 0.0) continue;
        vec2 uv = (clip.xy / clip.w) * 0.5 + 0.5;
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) continue;

        vec3 sceneP = pos(uv);
        // Optional: if your “empty” is (0,0,0), skip
        // if (all(lessThan(abs(sceneP), vec3(1e-6)))) continue;

        float scene_z = sceneP.z;

        // pick the correct sign for your view space (see note above)
        float angle_bias = max(in_bias, in_bias * (1.0 - dot(N, normalize(S - P))));
        float occluder   = (scene_z >= S.z + angle_bias) ? 1.0 : 0.0;

        float dist  = length(sceneP - S);
        float range = 1.0 - clamp(dist / in_radius, 0.0, 1.0);

        occlusion += occluder * range;
    }

    float ao = 1.0 - (occlusion / float(kernel_size));
    ao = pow(clamp(ao, 0.0, 1.0), in_power) * in_intensity;
    out_ssao_raw = ao;   // no debug lift
}
