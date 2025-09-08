// Version is specified in the prelude

layout (location = 0) in vec2 aPos;
layout (location = 1) in vec2 aTexCoords;

uniform mat4 in_model;
uniform mat4 in_projection;

out vec2 TexCoords;

void main()
{
    gl_Position = in_projection * in_model * vec4(aPos, 0.0, 1.0);
    TexCoords = aTexCoords;
}