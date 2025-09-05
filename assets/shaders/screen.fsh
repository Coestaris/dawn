#version 330 core
out vec4 FragColor;

in vec2 TexCoords;

uniform sampler2D color_texture;

void main()
{
    vec4 color = texture(color_texture, TexCoords);
    FragColor = vec4(color.rgb, 1.0);
}