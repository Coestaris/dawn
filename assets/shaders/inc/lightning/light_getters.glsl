// RGBA32, height 1
uniform usampler2D in_packed_lights;
// x=magic, y=ver, z=count, w=reserved
uniform uvec4 in_packed_lights_header;

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
