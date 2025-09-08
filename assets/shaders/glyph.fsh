// Version is specified in the prelude

out vec4 FragColor;
in vec2 TexCoords;

uniform vec4 in_color;
uniform sampler2D in_atlas;

void main()
{
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(in_atlas, TexCoords).r);
    FragColor = in_color * sampled;
}