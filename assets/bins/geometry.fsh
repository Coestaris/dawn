#version 330 core
out vec4 FragColor;

in vec2 TexCoords;
in vec3 Normal;

vec3 light_position = vec3(0.0, 0.0, 1.0); // Example light position

void main()
{
    // Calculate the light direction
    vec3 lightDir = normalize(light_position - vec3(TexCoords, 0.0)); // Using TexCoords as a placeholder for position
    // Calculate the normal vector (assuming Normal is already in world space)
    vec3 norm = normalize(Normal);
    // Calculate the diffuse lighting factor
    float diff = max(dot(norm, lightDir), 0.1);
    // Set the fragment color based on the diffuse lighting
    FragColor = vec4(diff * vec3(1.0, 1.0, 1.0), 1.0); // White color scaled by diffuse factor

    //    FragColor= vec4(1,1,1,1); // Set the fragment color to white
    //    FragColor = vec4(TexCoords, 0.0, 1.0); // Output the texture coordinates as color
    //    FragColor = vec4(Normal, 1.0); // Output the normal vector as color
    // FragColor = texture(texture_diffuse1, TexCoords); // Uncomment to use a
}