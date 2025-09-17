#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"

layout (location = 0) in vec3 aPos;

uniform mat4 in_model;

void main()
{
    gl_Position = in_projection * in_view * in_model * vec4(aPos, 1.0);
}