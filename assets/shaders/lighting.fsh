// Version is specified in the prelude

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

const uint LIGHT_KIND_DIRECTIONAL = 1u;
const uint LIGHT_KIND_POINT       = 2u;
const uint LIGHT_KIND_AREA_RECT   = 3u;

struct LightPacked {
    // x=kind, y=flags, z=reserved, w=float bits of intensity
    uvec4 kind_flags_intensity;
    // rgb=color, a=unused
    vec4 color_rgba;
    // sun: dir; point: pos.xyz, w=radius
    vec4 v0;
    // area: normal/halfHeight; others: reserved
    vec4 v1;
    // rough, metallic, falloff(0 phys / 1 lin), shadow
    vec4 brdf;
};

uint baseOf(uint idx) {
    return idx * 5u;
}

uvec4 fetch4(uint i) {
    return texelFetch(in_packed_lights, ivec2(int(i), 0), 0);
}

uint lightsCount() {
    return in_packed_lights_header.z;
}

LightPacked readLight(uint idx) {
    uint b = baseOf(idx);
    LightPacked L;
    L.kind_flags_intensity = fetch4(b + 0u);
    L.color_rgba           = uintBitsToFloat(fetch4(b + 1u));
    L.v0                   = uintBitsToFloat(fetch4(b + 2u));
    L.v1                   = uintBitsToFloat(fetch4(b + 3u));
    L.brdf                 = uintBitsToFloat(fetch4(b + 4u));
    return L;
}

uint lightKind(in LightPacked L) {
    return L.kind_flags_intensity.x;
}

uint lightFlags(in LightPacked L) {
    return L.kind_flags_intensity.y;
}

float lightIntensity(in LightPacked L) {
    return uintBitsToFloat(L.kind_flags_intensity.w);
}

vec3 lightColor(in LightPacked L) {
    return L.color_rgba.rgb;
}

vec3 lightSunDirection(in LightPacked L) {
    return normalize(-L.v0.xyz);
}

vec3 lightPointPosition(in LightPacked L) {
    return L.v0.xyz;
}

float lightPointRadius(in LightPacked L) {
    return L.v0.w;
}

float lightRoughness(in LightPacked L) {
    return L.brdf.x;
}

float lightMetallic(in LightPacked L) {
    return L.brdf.y;
}

bool lightFalloffLinear(in LightPacked L) {
    return L.brdf.z > 0.5;
}

bool lightCastsShadow(in LightPacked L) {
    return (L.brdf.w > 0.5);
}


// Decode a normal from an octahedral encoded vector
vec3 decode_oct(vec2 e) {
    vec3 v = vec3(e*2.0-1.0, 1.0 - abs(e.x*2.0-1.0) - abs(e.y*2.0-1.0));
    float t = clamp(-v.z, 0.0, 1.0);
    v.x += v.x >= 0.0 ? -t : t;
    v.y += v.y >= 0.0 ? -t : t;
    return normalize(v);
}

vec3 reconstruct_view_pos(float depth, vec2 uv, mat4 invProj, vec2 viewportSize) {
    float z = depth * 2.0 - 1.0;
    vec4 clip = vec4(uv*2.0-1.0, z, 1.0);
    vec4 view = invProj * clip;
    return view.xyz / view.w;
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

vec3 shade_sun(LightPacked L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    return vec3(0.2, 0.0, 0.0); // Placeholder
}

vec3 shade_point(LightPacked L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    vec3 light_position = lightPointPosition(L);
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
    vec3 light_color = lightColor(L);
    vec3 Lc = light_color * lightIntensity(L);

    // Attenuation
    float radius = lightPointRadius(L);
    bool linear = lightFalloffLinear(L);
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

vec3 shade_area_rect(LightPacked L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic) {
    return vec3(0.0, 1.0, 0.0); // Placeholder
}

vec4 process() {
    // Check magic and version
    if (in_packed_lights_header.x != 0x4C495445u) {
        return vec4(0.0, 1.0, 1.0, 1.0); // Cyan for invalid lights buffer
    }

    vec4 albedo_metallic = texture(in_albedo_metallic_texture, tex_coord);
    vec2 nor_oct = texture(in_normal_texture, tex_coord).rg;
    vec4 pbr = texture(in_pbr_texture, tex_coord);
    float depth = texture(in_depth_texture, tex_coord).r;

    vec3 N = decode_oct(nor_oct);
    float rough = max(pbr.r, 1.0/255.0);
    float metallic = albedo_metallic.a;
    float ao = pbr.g;
    vec3 albedo = albedo_metallic.rgb;
    vec3 P = reconstruct_view_pos(depth, tex_coord, in_inv_proj, in_viewport); // view-space
    vec3 V = normalize(-P);

    vec3 Lo = vec3(0);
    for (uint i = 0u; i < lightsCount(); i++) {
        LightPacked L = readLight(i);

        uint kind = lightKind(L);
        if (kind == LIGHT_KIND_DIRECTIONAL) {
            Lo += shade_sun(L, P, N, V, albedo, rough, metallic);
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
        FragColor = vec4(vec3(depth), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_POSITION) {
        float depth = texture(in_depth_texture, tex_coord).r;
        vec3 pos = reconstruct_view_pos(depth, tex_coord, in_inv_proj, in_viewport);
        FragColor = vec4(pos * 0.5 + 0.5, 1.0);
    } else {
        FragColor = vec4(1.0, 0.0, 1.0, 1.0); // Magenta for invalid mode
    }
}