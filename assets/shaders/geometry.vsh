// Version is specified in the prelude

layout (location = 0) in vec3 in_position;
layout (location = 1) in vec3 in_normal;
layout (location = 2) in vec2 in_tex_coord;

out mat4 model;
out mat4 view;
out vec2 tex_coord;
out vec3 normal;

uniform mat4 in_model;

void main()
{
    gl_Position = in_projection * in_view * in_model * vec4(in_position, 1.0);

    // Pass through the matrices and attributes to the fragment shader
    model = in_model;
    view = in_view;
    tex_coord = in_tex_coord;
    normal = in_normal;
}