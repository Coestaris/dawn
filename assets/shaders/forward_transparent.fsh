#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/normal.glsl"

// RGB8.
layout(location = 0) out vec4 out_color;

in vec2 tex_coord;
in vec3 normal;
in vec3 tangent;
in vec3 bitangent;
in vec3 view_pos;

uniform mat4 in_model;
uniform bool in_tangent_valid;

uniform float in_diffuse_scale;
uniform float in_specular_scale;

// RGB or RGBA
uniform sampler2D in_albedo;
// RGB
uniform sampler2D in_normal;
// R - metallic, G - roughness
uniform sampler2D in_metallic_roughness;
// R - occlusion
uniform sampler2D in_occlusion;

uniform samplerCube in_skybox;

#include "inc/lightning/light_getters.glsl"
#include "inc/lightning/pbr.glsl"

vec4 get_albedo() {
    return texture(in_albedo, tex_coord).rgba;
}

vec2 get_rm() {
    return texture(in_metallic_roughness, tex_coord).rg;
}

float get_occlusion() {
    return texture(in_occlusion, tex_coord).r;
}

vec3 get_normal()
{
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

    return normalize(n_view);
}

vec3 process(vec3 albedo, vec3 normal, float roughness, float metallic, float occlusion)
{
    // Check magic and version
#if ENABLE_DEVTOOLS
    if (in_packed_lights_header.x != 0x4C495445u) {
        return vec3(0.0, 1.0, 1.0); // Cyan for invalid lights buffer
    }
#endif

    vec3 P = view_pos;
    vec3 N = normalize(normal);
    vec3 V = normalize(-P); // View vector in view space
    float ao = occlusion;

    vec3 Lo = vec3(0.0);
    for (uint i = 0u; i < get_lights_count(); ++i) {
        PackedLight L = get_light(i);

        uint kind = get_light_kind(L);
        if (kind == LIGHT_KIND_SUN) {
            Lo += shade_sun(L, P, N, V, albedo, roughness, metallic, ao);
        } else if (kind == LIGHT_KIND_SPOT) {
            Lo += shade_spot(L, P, N, V, albedo, roughness, metallic, ao);
        } else if (kind == LIGHT_KIND_POINT) {
            Lo += shade_point(L, P, N, V, albedo, roughness, metallic, ao);
        } else if (kind == LIGHT_KIND_AREA_RECT) {
            Lo += shade_area_rect(L, P, N, V, albedo, roughness, metallic, ao);
        } else {
            // Unknown light kind. Output magenta to indicate error
            Lo += vec3(1.0, 0.0, 1.0);
        }
    }

    // Add IBL
    vec3 ambient = vec3(0.03) * albedo * ao;
    vec3 color = ambient + Lo;
    return color;
}

void main()
{
    vec4 albedo = get_albedo();
    if (albedo.a < 0.01)
    {
        // Barely visible, skip
        discard;
    }

    vec2 rm = get_rm();
    float occlusion = get_occlusion();
    vec3 normal = get_normal();

    vec3 color = process(albedo.rgb, normal, rm.x, rm.y, occlusion);
    out_color = vec4(color * albedo.a, albedo.a);
}