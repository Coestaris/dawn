#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec2 in_tex_coord;
layout (location = 3) in vec3 in_tangent;
layout (location = 4) in vec3 in_bitangent;

uniform mat4 in_model;

out vec2 tex_coord;
out vec3 normal;
out vec3 tangent;
out vec3 bitangent;
out vec3 view_pos;

void main()
{
    // Pass through the matrices and attributes to the fragment shader
    tex_coord = in_tex_coord;
    normal = in_normal;
    tangent = in_tangent;
    bitangent = in_bitangent;

    // Attention: This code MUST be the same as in the z_prepass.
    // otherwise depth will sligtly different causing
    // aggressive black artifacts
    vec4 vp = in_view * in_model * vec4(in_position, 1.0);
    view_pos = vp.xyz / vp.w;
    gl_Position = in_projection * vp;
}