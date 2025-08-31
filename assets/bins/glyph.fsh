#version 330 core

out vec4 FragColor;
in vec2 TexCoords;

uniform vec4 color;
uniform sampler2D atlas;

void main()
{
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(atlas, TexCoords).r);
    FragColor = color * sampled;
}