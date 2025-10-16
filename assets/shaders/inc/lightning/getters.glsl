
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

vec3 get_normal(vec2 uv) {
    return decode_oct(texture(in_normal, uv).rg);
}

float get_depth(vec2 uv) {
    return linearize_depth(texture(in_depth, uv).r, in_clip_planes.x, in_clip_planes.y);
}

vec3 get_pos(vec2 uv) {
    return reconstruct_view_pos(texture(in_depth, uv).r, uv, in_inv_proj);
}

vec3 get_albedo(vec2 uv) {
    return texture(in_albedo, uv).rgb;
}

vec3 get_orm(vec2 uv) {
    return texture(in_orm, uv).rgb;
}

vec3 get_skybox(vec2 uv) {
    // Calculate view direction
    vec3 view_pos = get_pos(uv);
    vec3 view_dir = normalize(view_pos);

    // Transform to world space
    vec3 world_dir = (in_inv_view * vec4(view_dir, 0.0)).xyz;

    return texture(in_skybox, world_dir).rgb;
}