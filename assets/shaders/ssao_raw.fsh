#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/depth.glsl"

uniform sampler2D in_position;
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

void main()
{
    vec2 noise_scale = vec2(ivec2(in_viewport) / textureSize(in_noise, 0));

    // Primary inputs
    vec3 position = texture(in_position, tex_coord).xyz;
    vec3 normal = normalize(texture(in_normal, tex_coord).xyz);
    vec3 random = normalize(texture(in_noise, tex_coord * noise_scale).xyz);

    // Make TBN matrix
    vec3 tangent = normalize(random - normal * dot(random, normal));
    vec3 bitangent = normalize(cross(normal, tangent));
    mat3 TBN = mat3(tangent, bitangent, normal);

    int kernel_size = int(in_kernel_size);

    float occlusion = 0.0;
    for (int i = 0; i < kernel_size; ++i) {
        // Get sample position
        vec3 sample_pos = TBN * in_samples[i].xyz; // From tangent to view-space
        sample_pos = position + sample_pos * in_radius;

        // project sample position (to sample texture) (to get position on screen/texture)
        vec4 offset = vec4(sample_pos, 1.0);
        offset = in_projection * offset; // from view to clip-space
        offset.xyz /= offset.w; // perspective divide
        offset.xyz = offset.xyz * 0.5 + 0.5; // transform to range 0.0 - 1.0

        // Get sample depth
        float sample_depth = texture(in_position, offset.xy).z; // Get depth value of kernel sample
        float range_check = smoothstep(0.0, 1.0, in_radius / abs(position.z - sample_depth));
        occlusion += (sample_depth >= sample_pos.z + in_bias ? 1.0 : 0.0) * range_check;
    }

    occlusion = 1.0 - (occlusion / in_kernel_size);
    occlusion = pow(occlusion, in_power);
    float ao = clamp(occlusion, 0.0, 1.0);

    out_raw_output = ao;
}