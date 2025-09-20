#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"



uniform sampler2D in_depth;
uniform sampler2D in_normal;
uniform sampler2D in_noise;
uniform sampler2D in_albedo;

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


out vec4 ssao_output; // RGB - color bleeding, A - occlusion

in vec2 tex_coord;

void main()
{

}