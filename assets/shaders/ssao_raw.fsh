#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

#define NOIZE_SCALE 128.0

uniform sampler2D in_depth;
uniform sampler2D in_normal;
uniform sampler2D in_noise;

layout(std140) uniform ubo_ssao_raw_params {
    float in_kernel_size;
    float in_radius;
    float in_bias;
    float in_intensity;
    float in_power;
    vec3 _padding;
};

layout(std140) uniform ubo_ssao_raw_kernel {
    vec4 in_samples[64];
};


out float raw_output; // occlusion

in vec2 tex_coord;

void n2tbn(in vec3 n, in vec3 rand, out vec3 t, out vec3 b) {
    t = normalize(rand - n * dot(rand, n));
    b = cross(n, t);
}

void main()
{
    vec2 uv = tex_coord;
    vec2 texel = in_viewport;

    float depth = texture(in_depth, uv).r;

    // Discard far plane fragments
    if (depth >= 1.0) {
        raw_output = 1.0;
        return;
    }

    vec3 P = reconstruct_view_pos(depth, uv, in_inv_proj);
    vec3 N = texture(in_normal, uv).xyz * 2.0 - 1.0;
    vec3 R = normalize(texture(in_noise, uv * NOIZE_SCALE).xyz * 2.0 - 1.0);

    vec3 T, B;
    n2tbn(N, R, T, B);
    mat3 TBN = mat3(T, B, N);

    float occlusion = 0.0;
    float weight = 0.0;

    for (int i = 0; i < int(in_kernel_size); ++i) {
        vec3 samp = TBN * in_samples[i].xyz; // From tangent to view-space
        vec3 Q = P + samp * in_radius;

        // Project sample position (view space) back to screen space
        vec4 Qc = in_projection * vec4(Q, 1.0);
        vec3 Qndc = Qc.xyz / Qc.w;
        vec2 Quv = Qndc.xy * 0.5 + 0.5;

        // Skip samples outside the screen
        if (Quv.x < 0.0 || Quv.x > 1.0 || Quv.y < 0.0 || Quv.y > 1.0)
            continue;

        float d01 = texture(in_depth, Quv).r;
        if (d01 >= 1.0)
            continue;

        // Current fragment view-space Z
        float Zv = linearize_depth(depth, in_clip_planes.x, in_clip_planes.y);
        float Qz = -Q.z; // TODO: ???

        float range_check = smoothstep(0.0, 1.0, in_radius / abs(Zv - Qz));
        float contribution = (Qz > Zv + in_bias) ? 1.0 : 0.0;
        occlusion += contribution * range_check;
        weight += range_check;
    }

    float ao = 1.0;
    if (weight > 0.0) {
        ao = 1.0 - (occlusion / weight) * in_intensity;
    }
    ao = pow(ao, in_power);

    // Output ambient occlusion factor
    raw_output = ao;
}