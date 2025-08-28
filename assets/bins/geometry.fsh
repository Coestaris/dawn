#version 330 core
out vec4 FragColor;

in vec2 TexCoords;
in vec3 Normal;

uniform sampler2D base_color_texture; // Base color texture

vec3 light_position = vec3(0.0, 0.0, 1.0); // Example light position

void main()
{
    // Calculate the light direction
    vec3 lightDir = normalize(light_position - vec3(TexCoords, 0.0)); // Using TexCoords as a placeholder for position
    // Calculate the normal vector (assuming Normal is already in world space)
    vec3 norm = normalize(Normal);
    // Calculate the diffuse lighting factor
    float diff = max(dot(norm, lightDir), 0.5);

    vec4 baseColor = texture(base_color_texture, TexCoords);
    baseColor.rgb *= diff;
    baseColor.rgb += 0.1;
    FragColor = vec4(baseColor.rgba);
}