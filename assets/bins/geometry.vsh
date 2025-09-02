#version 330 core
#extension GL_ARB_explicit_attrib_location : enable

layout (location = 0) in vec3 aPos;
layout (location = 1) in vec3 aNormal;
layout (location = 2) in vec2 aTexCoords;

out vec3 FragPos;
out vec2 TexCoords;
out vec3 Normal;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;

void main()
{
    // TODO: Transform to the screen space
    FragPos = vec3(model * vec4(aPos, 1.0));
    TexCoords = aTexCoords;
    // Transform normal to world space
    Normal = mat3(transpose(inverse(model))) * aNormal;

    gl_Position = projection * view * model * vec4(aPos, 1.0);

}