// Version is specified in the prelude

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec2 in_tex_coord;

out vec2 tex_coord;
out vec3 normal;

out vec3 view_position;

uniform mat4 in_model;

void main()
{
    gl_Position = in_projection * in_view * in_model * vec4(in_position, 1.0);

    // Pass through the matrices and attributes to the fragment shader
    tex_coord = in_tex_coord;
    normal = in_normal;
    view_position = vec3(in_view * in_model * vec4(in_position, 1.0));
}