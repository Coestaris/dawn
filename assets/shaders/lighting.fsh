#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/debug_mode.glsl"
#include "inc/normal.glsl"
#include "inc/depth.glsl"

out vec4 FragColor;

in vec2 tex_coord;

uniform sampler2D in_depth_texture;

// RGBA8. RGB - albedo, A - metallic
uniform sampler2D in_albedo_metallic_texture;
// RG16F. View space, Octa-encoded normal
uniform sampler2D in_normal_texture;
// RGBA8. R - roughness, G - occlusion, BA - reserved
uniform sampler2D in_pbr_texture;
// RGBA32, height 1
uniform usampler2D in_packed_lights;

// x=magic, y=ver, z=count, w=reserved
uniform uvec4 in_packed_lights_header;
// see inc/debug_mode.glsl
uniform int in_debug_mode;

#if ENABLE_DEVTOOLS

uniform vec3 ENV_SKY_COLOR;
uniform vec3 ENV_GROUND_COLOR;
uniform float ENV_DIFFUSE_SCALE;
uniform float ENV_SPECULAR_SCALE;

#else

const vec3 ENV_SKY_COLOR    = vec3(0.6, 0.7, 0.9);
const vec3 ENV_GROUND_COLOR = vec3(0.3, 0.25, 0.2);
const float ENV_DIFFUSE_SCALE  = 1.0;
const float ENV_SPECULAR_SCALE = 0.3;

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

uvec4 fetch4(uint i) {
    return texelFetch(in_packed_lights, ivec2(int(i), 0), 0);
}

uint get_lights_count() {
    return in_packed_lights_header.z;
}

PackedLight get_light(uint idx) {
    uint b = idx * 5u;
    PackedLight L;
    L.kind_flags_intensity = fetch4(b + 0u);
    L.color_rgba           = uintBitsToFloat(fetch4(b + 1u));
    L.v0                   = uintBitsToFloat(fetch4(b + 2u));
    L.v1                   = uintBitsToFloat(fetch4(b + 3u));
    L.brdf                 = uintBitsToFloat(fetch4(b + 4u));
    return L;
}

// 
// Common light accessors
//
uint get_light_kind(in PackedLight L) {
    return L.kind_flags_intensity.x;
}

uint get_light_flags(in PackedLight L) {
    return L.kind_flags_intensity.y;
}

float get_light_intensity(in PackedLight L) {
    return uintBitsToFloat(L.kind_flags_intensity.w);
}

vec3 get_light_color(in PackedLight L) {
    return L.color_rgba.rgb;
}

//
// Sun light accessors
//
vec3 get_light_sun_direction(in PackedLight L) {
    return normalize(L.v0.xyz);
}
float get_light_sun_ambient(in PackedLight L) {
    return L.v0.w;
}

//
// Point light accessors
//
vec3 get_light_point_position(in PackedLight L) {
    return L.v0.xyz;
}
float get_light_point_radius(in PackedLight L) {
    return L.v0.w;
}
bool get_light_point_falloff_linear(in PackedLight L) {
    return (L.brdf.z > 0.5);
}

//
// Spot light accessors
//
vec3 get_light_spot_position(in PackedLight L) {
    return L.v0.xyz;
}
float get_light_spot_range(in PackedLight L) {
    return L.v0.w;
}
vec3 get_light_spot_direction(in PackedLight L) {
    return normalize(L.v1.xyz);
}
float get_light_spot_inner_cone(in PackedLight L) {
    return L.v1.w;
}
float get_light_spot_outer_cone(in PackedLight L) {
    return L.color_rgba.a;
}

// Simple helpers
float saturate(float x) {
    return clamp(x, 0.0, 1.0);
}

vec3 saturate3(vec3 v) {
    return clamp(v, 0.0, 1.0);
}

float D_GGX(float NoH, float a) {
    float a2 = a*a;
    float d = (NoH*NoH) * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * d * d + 1e-5);
}

float V_SmithGGXCorrelated(float NoV, float NoL, float a) {
    float a2 = a*a;
    float gv = NoL * sqrt((-NoV*a2 + NoV) * NoV + a2);
    float gl = NoV * sqrt((-NoL*a2 + NoL) * NoL + a2);
    return 0.5 / (gv + gl + 1e-5);
}

vec3 F_Schlick(vec3 F0, float HoV){
    return F0 + (1.0-F0)*pow(1.0 - HoV, 5.0);
}

vec3 brdf_lambert(vec3 albedo, float metallic){
    // energy-conserving: diffuse*(1-metallic)
    return albedo * (1.0 - metallic) / 3.14159265;
}

float point_atten(float d, float radius, bool linear){
    if (d > radius) return 0.0;
    if (linear) {
        // Linear falloff to zero at radius
        return 1.0 - d / radius;
    } else {
        // Physically based quadratic falloff
        float att = 1.0 / (d * d);
        // Normalize so that att(0) = 1 and att(radius) = 0
        float att_radius = 1.0 / (radius * radius);
        return att / (att + att_radius);
    }
}

vec3 shade_point(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    vec3 light_position = get_light_point_position(L);
    // Vector from surface point to light
    vec3 Lvec = (light_position - P);
    float d2 = dot(Lvec, Lvec);
    float d = sqrt(d2);
    // Direction from surface point to light
    vec3 Ldir = Lvec / max(d, 1e-5);
    // NoL - cosine between normal and light direction
    float NoL = max(dot(N, Ldir), 0.0);
    // If light is below the horizon, skip
    if (NoL <= 0.0) return vec3(0);

    // Light color and intensity
    vec3 light_color = get_light_color(L);
    vec3 Lc = light_color * get_light_intensity(L);

    // Attenuation
    float radius = get_light_point_radius(L);
    bool linear = get_light_point_falloff_linear(L);
    float atten = point_atten(d, radius, linear);
    // If fully attenuated, skip to not waste computations
    if (atten <= 0.0) return vec3(0);

    // Cook-Torrance BRDF
    vec3 H = normalize(V + Ldir);
    float NoV = max(dot(N, V), 1e-4);
    float NoH = max(dot(N, H), 1e-4);
    float HoV = max(dot(H, V), 1e-4);
    float a = max(rough*rough, 1e-4);
    // Fresnel at normal incidence
    vec3 F0 = mix(vec3(0.04), albedo, metallic);
    float D = D_GGX(NoH, a);
    float Vg = V_SmithGGXCorrelated(NoV, NoL, a);
    vec3  F = F_Schlick(F0, HoV);
    // Specular and diffuse terms
    vec3 spec = (D*Vg) * F;
    vec3 diff = brdf_lambert(albedo, metallic);
    return (diff + spec) * Lc * (NoL * atten);
}

vec3 shade_sun(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    N = normalize(N);
    V = normalize(V);

    vec3  Ldir = -get_light_sun_direction(L);
    float NoL  = max(dot(N, Ldir), 0.0);
    if (NoL <= 0.0) {
    }

    vec3 light_color = get_light_color(L);
    vec3 Lc = light_color * get_light_intensity(L);

    float a = max(rough * rough, 1e-4);
    vec3  H = normalize(V + Ldir);
    float NoV = max(dot(N, V), 1e-4);
    float NoH = max(dot(N, H), 1e-4);
    float HoV = max(dot(H, V), 1e-4);

    vec3 F0  = mix(vec3(0.04), albedo, metallic);

    float D   = D_GGX(NoH, a);
    float Vg  = V_SmithGGXCorrelated(NoV, NoL, a);
    vec3  F   = F_Schlick(F0, HoV);
    float ao  = 1.0;// TODO: get from texture

    vec3  diff = brdf_lambert(albedo, metallic);
    vec3  spec = (D * Vg) * F;

    vec3 Lo_direct = (NoL > 0.0) ? (diff + spec) * Lc * NoL : vec3(0.0);

    float ambSun = get_light_sun_ambient(L);
    float NoUp = clamp(dot(N, normalize(ENV_UP)) * 0.5 + 0.5, 0.0, 1.0);
    vec3 hemiIrradiance = mix(ENV_GROUND_COLOR, ENV_SKY_COLOR, NoUp) * ambSun * ENV_DIFFUSE_SCALE;
    vec3 ambientDiffuse = albedo * hemiIrradiance * (1.0 - metallic) * ao;

    float avgF0 = clamp((F0.x + F0.y + F0.z) * (1.0 / 3.0), 0.0, 1.0);
    ambientDiffuse *= (1.0 - 0.25 * avgF0);

    vec3 F_amb = F_Schlick(F0, NoV);
    float roughAtten = mix(1.0, 0.5, clamp(rough, 0.0, 1.0));
    vec3 specAmb = F_amb * ambSun * ENV_SPECULAR_SCALE * roughAtten * ao;

    // Итог
    return Lo_direct + ambientDiffuse + specAmb;
}

vec3 shade_spot(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    return vec3(0.0, 0.2, 0.0);// Placeholder
}

vec3 shade_area_rect(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    return vec3(0.0, 1.0, 0.0);// Placeholder
}

vec4 process() {
    // Check magic and version
    #if ENABLE_DEVTOOLS
    if (in_packed_lights_header.x != 0x4C495445u) {
        return vec4(0.0, 1.0, 1.0, 1.0);// Cyan for invalid lights buffer
    }
    #endif

    vec4 albedo_metallic = texture(in_albedo_metallic_texture, tex_coord);
    vec2 nor_oct = texture(in_normal_texture, tex_coord).rg;
    vec4 pbr = texture(in_pbr_texture, tex_coord);
    float depth = texture(in_depth_texture, tex_coord).r;

    vec3 N = decode_oct(nor_oct);
    float rough = max(pbr.r, 1.0/255.0);
    float metallic = albedo_metallic.a;
    float ao = pbr.g;
    vec3 albedo = albedo_metallic.rgb;
    vec3 P = reconstruct_view_pos(depth, tex_coord, in_inv_proj);// view-space
    vec3 V = normalize(-P);

    vec3 Lo = vec3(0);
    for (uint i = 0u; i < get_lights_count(); ++i) {
        PackedLight L = get_light(i);

        uint kind = get_light_kind(L);
        if (kind == LIGHT_KIND_SUN) {
            Lo += shade_sun(L, P, N, V, albedo, rough, metallic);
        } else if (kind == LIGHT_KIND_SPOT) {
            Lo += shade_spot(L, P, N, V, albedo, rough, metallic);
        } else if (kind == LIGHT_KIND_POINT) {
            Lo += shade_point(L, P, N, V, albedo, rough, metallic);
        } else if (kind == LIGHT_KIND_AREA_RECT) {
            Lo += shade_area_rect(L, P, N, V, albedo, rough, metallic);
        } else {
            // Unknown light kind. Output magenta to indicate error
            Lo += vec3(1.0, 0.0, 1.0);
        }
    }

    Lo = mix(Lo * ao, Lo, metallic);
    vec3 ambient = vec3(0.03) * albedo * ao;
    vec3 color = ambient + Lo;

    return vec4(color, 1.0);
}

void main()
{
#if ENABLE_DEVTOOLS
    if (in_debug_mode == DEBUG_MODE_OFF) {
        FragColor = process();
    } else if (in_debug_mode == DEBUG_MODE_ALBEDO) {
        FragColor = texture(in_albedo_metallic_texture, tex_coord);
    } else if (in_debug_mode == DEBUG_MODE_METALLIC) {
        float metallic = texture(in_albedo_metallic_texture, tex_coord).a;
        FragColor = vec4(vec3(metallic), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_NORMAL) {
        vec2 oct_normal = texture(in_normal_texture, tex_coord).rg;
        vec3 normal = decode_oct(oct_normal);
        FragColor = vec4(normal * 0.5 + 0.5, 1.0);
    } else if (in_debug_mode == DEBUG_MODE_ROUGHNESS) {
        float roughness = texture(in_pbr_texture, tex_coord).r;
        FragColor = vec4(vec3(roughness), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_AO) {
        float ao = texture(in_pbr_texture, tex_coord).g;
        FragColor = vec4(vec3(ao), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_DEPTH) {
        // Reconstruct linear depth [0..1]
        float depth = texture(in_depth_texture, tex_coord).r;
        float linear = linearize_depth(depth, in_clip_planes.x, in_clip_planes.y) / in_clip_planes.y;
        FragColor = vec4(vec3(linear), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_POSITION) {
        float depth = texture(in_depth_texture, tex_coord).r;
        vec3 pos = reconstruct_view_pos(depth, tex_coord, in_inv_proj);
        FragColor = vec4(pos * 0.5 + 0.5, 1.0);
    } else {
        FragColor = vec4(1.0, 0.0, 1.0, 1.0);// Magenta for invalid mode
    }
#else
    FragColor = process();
#endif
}