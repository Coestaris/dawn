#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

#define NOIZE_SCALE 32.0

uniform sampler2D in_depth;
uniform sampler2D in_normal;
uniform sampler2D in_noise;

// TODO: Hardcode values for non-devtools build

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


out float out_raw_output;

in vec2 tex_coord;

void n2tbn(in vec3 n, in vec3 rand, out vec3 t, out vec3 b) {
    t = normalize(rand - n * dot(rand, n));
    b = cross(n, t);
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
    int kernel_size = int(in_kernel_size);
    vec2 noise_scale = vec2(ivec2(in_viewport) / textureSize(in_noise, 0));

    float depth = texture(in_depth, uv).r;

    // Discard far plane fragments
    if (depth >= 1.0) {
        out_raw_output = 1.0;
        return;
    }

    vec3 P = reconstruct_view_pos(depth, uv, in_inv_proj);// view-space
    vec3 N = normal(uv); // view-space
    vec3 R = normalize(texture(in_noise, uv * noise_scale).xyz);

    vec3 Vdir = normalize(-P);
    float NoV = clamp(dot(N, Vdir), 0.0, 1.0);
    float fresnel = mix(0.35, 1.0, NoV);

    vec3 T, B;
    n2tbn(N, R, T, B);
    mat3 TBN = mat3(T, B, N);

    float occlusion = 0.0;
    float weight = 0.0;

    float fovy      = 2.0 * atan(1.0 / in_projection[1][1]);
    vec2  px2vs     = (2.0 * -P.z * tan(0.5 * fovy)) / in_viewport;
    float radiusVS  = in_radius * px2vs.y;
    float biasVS    = in_bias   * radiusVS;

    for (int i = 0; i < kernel_size; ++i) {
        vec3 sampVS = TBN * in_samples[i].xyz * radiusVS;
        vec3 Q = P + sampVS; // sample in view-space

        vec4 Qc = in_projection * vec4(Q, 1.0);
        if (Qc.w <= 0.0) {
            // Sample behind the near plane
            continue;
        };

        vec3 Qndc = Qc.xyz / Qc.w;
        if (any(lessThan(Qndc.xy, vec2(-1.0))) || any(greaterThan(Qndc.xy, vec2(1.0)))) {
            // Skip samples outside the NDC cube
            continue;
        }

        vec2 Quv = Qndc.xy * 0.5 + 0.5;
        if (Quv.x < 0.0 || Quv.x > 1.0 || Quv.y < 0.0 || Quv.y > 1.0) {
            // Outside the texture
            continue;
        }

//        vec2 nrg = texture(in_normal, Quv).rg;
//        vec3 Ns  = decode_oct(nrg);                  // view-space
//        float nDot = clamp(dot(N, Ns), 0.0, 1.0);
//        if (nDot < 0.2) {
//            continue;
//        }
//        float nWeight = nDot * nDot;

        float d01 = texture(in_depth, Quv).r;
        if (d01 >= 1.0) {
            // Far plane
            continue;
        }

        float zScene = linearize_depth(d01, in_clip_planes.x, in_clip_planes.y);// < 0
        float zSamp  = Q.z;// < 0

        float dz = zScene - zSamp;
        if (dz < biasVS) {
            // Too close
            continue;
        }

        float distVS = length(sampVS);
        float range  = 1.0 - clamp(dz / (distVS + 1e-4), 0.0, 1.0);
        range = range * range * fresnel;

        occlusion += range;
        weight    += 1.0;
    }

    float ao = 1.0;
    if (weight > 0.0) {
        ao = 1.0 - (occlusion / weight) * in_intensity;
    }

    ao = pow(ao, in_power);

    out_raw_output = ao;
}