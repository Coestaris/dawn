#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"

layout (location = 0) in vec3 in_position;

uniform mat4 in_model;

void main()
{
    gl_Position = in_projection * in_view * in_model * vec4(in_position, 1.0);
}