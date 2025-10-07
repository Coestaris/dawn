#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/normal.glsl"

// RGB8.
layout (location = 0) out vec3 out_albedo;
// RGB8. R - occlusion, G - roughness, B - metallic
layout (location = 1) out vec3 out_orm;
// RG8_SNORM. Octo encoded normal, view space
layout (location = 2) out vec2 out_normal;

in vec2 tex_coord;
in vec3 normal;
in vec3 tangent;
in vec3 bitangent;

uniform mat4 in_model;
uniform bool in_tangent_valid;

// RGB or RGBA
uniform sampler2D in_albedo;
// RGB
uniform sampler2D in_normal;
// R - metallic, G - roughness
uniform sampler2D in_metallic_roughness;
// R - occlusion
uniform sampler2D in_occlusion;

void main()
{
    vec3 albedo = texture(in_albedo, tex_coord).rgb;
    float roughness = texture(in_metallic_roughness, tex_coord).r;
    float metallic = texture(in_metallic_roughness, tex_coord).g;
    float occlusion = texture(in_occlusion, tex_coord).r;

    vec3 n_view;
    vec3 n_model_geo = normalize(normal);
    mat3 n_matrix = transpose(inverse(mat3(in_view * in_model)));
    if (in_tangent_valid)
    {
        vec3 n_tangent = texture(in_normal, tex_coord).rgb * 2.0 - 1.0;
        vec3 T = normalize(tangent);
        vec3 B = normalize(bitangent);
        vec3 N = normalize(n_model_geo);
        mat3 TBN = mat3(T, B, N);
        vec3 n_model = normalize(TBN * n_tangent);
        n_view = n_matrix * n_model;
    }
    else
    {
        n_view = n_matrix * n_model_geo;
    }

    out_albedo = albedo;
    out_orm = vec3(occlusion, roughness, metallic);
    out_normal = encode_oct(normalize(n_view));
}