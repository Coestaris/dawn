#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"

layout (location = 0) in vec2 in_box;
layout (location = 1) in vec2 in_tex_coord;

out vec2 tex_coord;

uniform vec2 in_size;
uniform vec3 in_position;

void main()
{
    // Calculate the right and up vectors from the view matrix
    mat3 R = transpose(mat3(in_view));
    vec3 right = R[0];
    vec3 up = R[1];

    // Calculate the world position of the billboard vertex
    vec3 world_position = in_position +
        (right * in_box.x * in_size.x) +
        (up * in_box.y * in_size.y);

    gl_Position = in_projection * in_view * vec4(world_position, 1.0);
    tex_coord = in_tex_coord;
}
