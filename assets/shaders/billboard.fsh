// Version is specified in the prelude

out vec4 FragColor;

in vec2 tex_coord;

uniform sampler2D in_sprite;

void main()
{
    FragColor = texture(in_sprite, tex_coord);
}