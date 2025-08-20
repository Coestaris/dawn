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
    float diff = max(dot(norm, lightDir), 0.0);

    // Sample the base color texture
    vec4 baseColor = texture(base_color_texture, TexCoords);
    // Apply the diffuse lighting to the base color
    FragColor = vec4(baseColor.rgb * diff, baseColor.a);
    // Optionally, you can add ambient lighting or other effects here
    // For example, adding a simple ambient light:
    vec3 ambient = 0.1 * baseColor.rgb; // Simple ambient light
}