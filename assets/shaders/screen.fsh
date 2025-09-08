// Version is specified in the prelude

out vec4 FragColor;

in vec2 TexCoords;

// RGBA8. RGB - albedo, A - metallic
uniform sampler2D in_albedo_metallic_texture;
// RG16F. View space, Octa-encoded normal
uniform sampler2D in_normal_texture;
// RGBA8. R - roughness, G - occlusion, BA - reserved
uniform sampler2D in_pbr_texture;

#define DEBUG_MODE_OFF 0
#define DEBUG_MODE_ALBEDO 1
#define DEBUG_MODE_METALLIC 2
#define DEBUG_MODE_NORMAL 3
#define DEBUG_MODE_ROUGHNESS 4
#define DEBUG_MODE_AO 5

// 0 = off,
// 1 - display albedo
// 2 - display metallic
// 3 - display normal
// 4 - display roughness
// 5 - display ao
uniform int in_debug_mode;

// Decode a normal from an octahedral encoded vector
vec3 decode_oct(vec2 e) {
    vec3 v = vec3(e*2.0-1.0, 1.0 - abs(e.x*2.0-1.0) - abs(e.y*2.0-1.0));
    float t = clamp(-v.z, 0.0, 1.0);
    v.x += v.x >= 0.0 ? -t : t;
    v.y += v.y >= 0.0 ? -t : t;
    return normalize(v);
}

vec4 process() {
    // TODO: Implement real fragment processing
    return texture(in_albedo_metallic_texture, TexCoords);
}

void main()
{
    if (in_debug_mode == DEBUG_MODE_OFF) {
        FragColor = process();
    } else if (in_debug_mode == DEBUG_MODE_ALBEDO) {
        FragColor = texture(in_albedo_metallic_texture, TexCoords);
    } else if (in_debug_mode == DEBUG_MODE_METALLIC) {
        float metallic = texture(in_albedo_metallic_texture, TexCoords).a;
        FragColor = vec4(vec3(metallic), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_NORMAL) {
        vec2 oct_normal = texture(in_normal_texture, TexCoords).rg;
        vec3 normal = decode_oct(oct_normal);
        FragColor = vec4(normal * 0.5 + 0.5, 1.0);
    } else if (in_debug_mode == DEBUG_MODE_ROUGHNESS) {
        float roughness = texture(in_pbr_texture, TexCoords).r;
        FragColor = vec4(vec3(roughness), 1.0);
    } else if (in_debug_mode == DEBUG_MODE_AO) {
        float ao = texture(in_pbr_texture, TexCoords).g;
        FragColor = vec4(vec3(ao), 1.0);
    } else {
        FragColor = vec4(1.0, 0.0, 1.0, 1.0); // Magenta for invalid mode
    }
}