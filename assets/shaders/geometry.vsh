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

void main()
{
    gl_Position = in_projection * in_view * in_model * vec4(in_position, 1.0);

    // Pass through the matrices and attributes to the fragment shader
    tex_coord = in_tex_coord;
    normal = in_normal;
    tangent = in_tangent;
    bitangent = in_bitangent;
}