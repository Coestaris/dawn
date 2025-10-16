#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

layout(location = 0) out vec3 out_color;

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
// RGBA32, height 1
uniform usampler2D in_packed_lights;
// x=magic, y=ver, z=count, w=reserved
uniform uvec4 in_packed_lights_header;

uniform samplerCube in_skybox;

#if ENABLE_DEVTOOLS

uniform vec3  in_sky_color;
uniform vec3  in_ground_color;
uniform float in_diffuse_scale;
uniform float in_specular_scale;
uniform int  in_ssao_enabled;

// see inc/debug_mode.glsl
uniform int  in_debug_mode;

#else

const vec3 in_sky_color       = DEF_SKY_COLOR;
const vec3 in_ground_color    = DEF_GROUND_COLOR;
const float in_diffuse_scale  = DEF_DIFFUSE_SCALE;
const float in_specular_scale = DEF_SPECULAR_SCALE;
const int in_ssao_enabled     = DEF_SSAO_ENABLED;
const int in_debug_mode       = DEBUG_MODE_OFF;

#endif

const vec3 ENV_UP = vec3(0.0, 1.0, 0.0);

const uint LIGHT_KIND_SUN       = 1u;
const uint LIGHT_KIND_SPOT      = 2u;
const uint LIGHT_KIND_POINT     = 3u;
const uint LIGHT_KIND_AREA_RECT = 4u;

struct PackedLight {
    // x=kind, y=flags, z=reserved, w=float bits of intensity
    uvec4 kind_flags_intensity;

    // sun: rgb
    // spot: rgb, a=outer angle (cosine)
    // point: rgb, a=unused
    vec4 color_rgba;

    // sun: dir.xyz, w=ambient
    // spot: pos.xyz, w=range
    // point: pos.xyz, w=radius
    vec4 v0;

    // sun: unused
    // spot: dir.xyz, w=inner angle (cosine)
    // point: unused
    vec4 v1;

    // rough, metallic, falloff(0 phys / 1 lin), shadow
    vec4 brdf;
};

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
    vec2 uv = (gl_FragCoord.xy + 0.5) / vec2(textureSize(in_depth, 0));
    
#if ENABLE_DEVTOOLS
    if (in_debug_mode == DEBUG_MODE_OFF) {
        out_color = process(uv);
    } else if (in_debug_mode == DEBUG_MODE_ALBEDO) {
        out_color = vec3(get_albedo(uv));
    } else if (in_debug_mode == DEBUG_MODE_METALLIC) {
        float metallic = get_orm(uv).b;
        out_color = vec3(metallic);
    } else if (in_debug_mode == DEBUG_MODE_NORMAL) {
        vec3 normal = get_normal(uv);
        out_color = vec3(normal * 0.5 + 0.5);
    } else if (in_debug_mode == DEBUG_MODE_ROUGHNESS) {
        float roughness = get_orm(uv).g;
        out_color = vec3(roughness);
    } else if (in_debug_mode == DEBUG_MODE_AO) {
        float ao = get_orm(uv).r;
        out_color = vec3(ao);
    } else if (in_debug_mode == DEBUG_MODE_DEPTH) {
        float d = get_depth(uv);
        out_color = vec3(d);
    } else if (in_debug_mode == DEBUG_MODE_POSITION) {
        vec3 p = get_pos(uv);
        out_color = vec3(p);
    } else if (in_debug_mode == DEBUG_MODE_SSAO) {
        float ao = get_ssao(uv);
        out_color = vec3(ao);
    } else if (in_debug_mode == DEBUG_MODE_SKYBOX) {
        vec3 color = get_skybox(uv);
        out_color = color;
    } else {
        out_color = vec3(1.0, 0.0, 1.0); // Magenta for unknown debug mode
    }
#else
    out_color = process(uv);
#endif
}