// Version is specified in the prelude

// RGBA8. RGB - albedo, A - metallic
layout (location = 0) out vec4 out_albedo_metalic;
// RG16F. View space, Octa-encoded normal
layout (location = 1) out vec2 out_normal_texture;
// RGBA8. R - roughness, G - occlusion, BA - reserved
layout (location = 2) out vec4 out_pbr;

in vec2 tex_coord;
in vec3 normal;
in vec3 tangent;
in vec3 bitangent;

uniform mat4 in_model;
uniform sampler2D in_albedo;
uniform sampler2D in_normal;
uniform sampler2D in_metallic;
uniform sampler2D in_roughness;
uniform sampler2D in_occlusion;

// Encode a normal into an octahedral encoded vector
vec2 encode_oct(vec3 n) {
    n /= (abs(n.x) + abs(n.y) + abs(n.z));
    vec2 enc = n.xy;
    if (n.z < 0.0) {
        enc = (1.0 - vec2(abs(enc.y), abs(enc.x))) * vec2(sign(enc.x), sign(enc.y));
    }
    return enc * 0.5 + 0.5;
}

void main()
{
    vec3 albedo = texture(in_albedo, tex_coord).rgb;
    float metallic = texture(in_metallic, tex_coord).r;
    float roughness = texture(in_roughness, tex_coord).r;
    float occlusion = texture(in_occlusion, tex_coord).r;

    vec3 n_model_geo = normalize(normal);
    vec3 n_tangent = texture(in_normal, tex_coord).rgb * 2.0 - 1.0;
    vec3 T = normalize(tangent);
    vec3 B = normalize(bitangent);
    vec3 N = normalize(n_model_geo);
    mat3 TBN = mat3(T, B, N);
    vec3 n_model = normalize(TBN * n_tangent);

    mat3 N_model = transpose(inverse(mat3(in_model)));
    vec3 n_world_or_modelfixed = normalize(N_model * n_model);
    vec3 n_view = normalize(mat3(in_view) * n_world_or_modelfixed);
    vec2 oct_normal = encode_oct(n_view);

    out_albedo_metalic = vec4(albedo, metallic);
    out_normal_texture = oct_normal;
    out_pbr = vec4(roughness, occlusion, 0.0, 0.0);
}