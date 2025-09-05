#version 330 core

// RGBA8. RGB - albedo, A - metallic
layout (location = 0) out vec4 out_albedo_metalic;
// RG16F. View space, Octa-encoded normal
layout (location = 1) out vec2 out_normal_texture;
// RGBA8. R - roughness, G - occlusion, BA - reserved
layout (location = 2) out vec4 out_pbr;

in mat4 model;
in mat4 view;
in vec2 tex_coord;
in vec3 normal;

uniform sampler2D in_albedo;
uniform sampler2D in_normal;
uniform sampler2D in_metallic;
uniform sampler2D in_roughness;
uniform sampler2D in_occlusion;

// Encode a normal into an octahedral encoded vector
vec2 encode_octahedron(vec3 n) {
    return n.z < 0.0 ?
    (n.xy / (abs(n.x) + abs(n.y)) * (1.0 - abs(n.z)) + 1.0) * 0.5 :
    n.xy * 0.5 + 0.5;
}

// Transform a normal from model space to view space
vec3 to_view_space(vec3 n, mat4 model, mat4 view) {
    mat3 normal_matrix = transpose(inverse(mat3(model)));
    return normalize(view * vec4(normal_matrix * n, 0.0)).xyz;
}

void main()
{
    vec3 albedo = texture(in_albedo, tex_coord).rgb;
    float metallic = texture(in_metallic, tex_coord).r;
    float roughness = texture(in_roughness, tex_coord).r;
    float occlusion = texture(in_occlusion, tex_coord).r;

    vec3 tex_normal = texture(in_normal, tex_coord).rgb;
    // Join two normal maps
    tex_normal = normalize(tex_normal * 2.0 - 1.0);
    vec3 normal = normalize(normal + tex_normal);

    // Transform normal from tangent to view space
    vec3 view_normal = to_view_space(normal, model, view);
    // Encode normal to octahedron
    vec2 oct_normal = encode_octahedron(view_normal);

    out_albedo_metalic = vec4(albedo, metallic);
    out_normal_texture = oct_normal;
    out_pbr = vec4(roughness, occlusion, 0.0, 0.0);
}