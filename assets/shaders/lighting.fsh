#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

layout(location = 0) out vec4 out_color;

// DEPTH24. OpenGL default depth format
uniform sampler2D in_depth;
// RGB8.
uniform sampler2D in_albedo;
// RGB8. R - occlusion, G - roughness, B - metallic
uniform sampler2D in_orm;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_normal;
// R8
uniform sampler2D in_halfres_ssao;

// NOTE: Some of samplers defined in light_getters.glsl

uniform samplerCube in_skybox;

#if ENABLE_DEVTOOLS

uniform float in_diffuse_scale;
uniform float in_specular_scale;
uniform int  in_ssao_enabled;

// see inc/debug_mode.glsl
uniform int  in_debug_mode;

#else

const float in_diffuse_scale  = DEF_DIFFUSE_SCALE;
const float in_specular_scale = DEF_SPECULAR_SCALE;
const int in_ssao_enabled     = DEF_SSAO_ENABLED;
const int in_debug_mode       = DEBUG_MODE_OFF;

#endif

#include "inc/lightning/light_getters.glsl"
#include "inc/lightning/getters.glsl"
#include "inc/lightning/ssao_upscale.glsl"
#include "inc/lightning/pbr.glsl"

vec3 process(vec2 uv) {
    // Check magic and version
#if ENABLE_DEVTOOLS
    if (in_packed_lights_header.x != 0x4C495445u) {
        return vec3(0.0, 1.0, 1.0); // Cyan for invalid lights buffer
    }
#endif

    // Fetch values from textures
    float linear_depth = get_depth(uv);
    if (linear_depth >= in_clip_planes.y) {
        // Far plane, return skybox color
        return get_skybox(uv);
    }

    float ssao = get_ssao(uv);
    vec3 orm = get_orm(uv);
    vec3 albedo = get_albedo(uv);
    vec3 P = get_pos(uv);
    vec3 N = get_normal(uv);

    // Calculate lighting
    float occlusion = orm.r;
    float roughness = orm.g;
    float metallic = orm.b;
    vec3 V = normalize(-P);
    vec3 Lo = vec3(0);

    // Ambient occlusion
    float ao = mix(1.0, occlusion * ssao, 1.0);

    for (uint i = 0u; i < get_lights_count(); ++i) {
        PackedLight L = get_light(i);

        uint kind = get_light_kind(L);
        if (kind == LIGHT_KIND_SUN) {
            Lo += shade_sun(L, P, N,    V, albedo, roughness, metallic, ao);
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
    vec2 uv = (gl_FragCoord.xy + 0.5) / vec2(textureSize(in_depth, 0));

    vec3 color = vec3(0.0);
#if ENABLE_DEVTOOLS
    if (in_debug_mode == DEBUG_MODE_OFF) {
        color = process(uv);
    } else if (in_debug_mode == DEBUG_MODE_ALBEDO) {
        color = vec3(get_albedo(uv));
    } else if (in_debug_mode == DEBUG_MODE_METALLIC) {
        float metallic = get_orm(uv).b;
        color = vec3(metallic);
    } else if (in_debug_mode == DEBUG_MODE_NORMAL) {
        vec3 normal = get_normal(uv);
        color = vec3(normal * 0.5 + 0.5);
    } else if (in_debug_mode == DEBUG_MODE_ROUGHNESS) {
        float roughness = get_orm(uv).g;
        color = vec3(roughness);
    } else if (in_debug_mode == DEBUG_MODE_AO) {
        float ao = get_orm(uv).r;
        color = vec3(ao);
    } else if (in_debug_mode == DEBUG_MODE_DEPTH) {
        float d = get_depth(uv);
        color = vec3(d);
    } else if (in_debug_mode == DEBUG_MODE_POSITION) {
        vec3 p = get_pos(uv);
        color = vec3(p);
    } else if (in_debug_mode == DEBUG_MODE_SSAO) {
        float ao = get_ssao(uv);
        color = vec3(ao);
    } else if (in_debug_mode == DEBUG_MODE_SKYBOX) {
        vec3 color = get_skybox(uv);
        color = color;
    } else {
        color = vec3(1.0, 0.0, 1.0); // Magenta for unknown debug mode
    }
#else
    color = process(uv);
#endif

    out_color = vec4(color, 1.0);
}