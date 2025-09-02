#version 330 core
layout (location = 0) out vec3 gPosition;
layout (location = 1) out vec3 gNormal;
layout (location = 2) out vec4 gAlbedoSpec;

in vec2 TexCoords;
in vec3 FragPos;
in vec3 Normal;

uniform sampler2D base_color_texture;// Base color texture

vec3 light_position = vec3(0.0, 0.0, 1.0);// Example light position

vec3 albedo() {
    // Calculate the light direction
    vec3 lightDir = normalize(light_position - vec3(TexCoords, 0.0));// Using TexCoords as a placeholder for position
    // Calculate the normal vector (assuming Normal is already in world space)
    vec3 norm = normalize(Normal);
    // Calculate the diffuse lighting factor
    float diff = max(dot(norm, lightDir), 0.5);

    vec4 baseColor = texture(base_color_texture, TexCoords);
    baseColor.rgb *= diff;
    baseColor.rgb += 0.1;
    return baseColor.rgb;
}

float specular() {
    // TODO: Implement specular calculation
    return 0.5;
}

void main()
{
    // store the fragment position vector in the first gbuffer texture
    gPosition = FragPos;
    // also store the per-fragment normals into the gbuffer
    gNormal = normalize(Normal);
    // and the diffuse per-fragment color
    gAlbedoSpec.rgb = albedo();
    // store specular intensity in gAlbedoSpec's alpha component
    gAlbedoSpec.a = specular();
}