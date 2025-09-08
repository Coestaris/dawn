
// Decode a normal from an octahedral encoded vector
vec3 decode_oct(vec2 e) {
    vec3 v = vec3(e*2.0-1.0, 1.0 - abs(e.x*2.0-1.0) - abs(e.y*2.0-1.0));
    float t = clamp(-v.z, 0.0, 1.0);
    v.x += v.x >= 0.0 ? -t : t;
    v.y += v.y >= 0.0 ? -t : t;
    return normalize(v);
}

// Reconstruct view space position from depth
// invProj: inverse of projection matrix
// uv: texture coordinates (0 to 1)
// depth: depth value (0 to 1)
// viewportSize: size of the viewport in pixels
vec3 reconstruct_view_pos(float depth, vec2 uv, mat4 invProj, vec2 viewportSize) {
    float z = depth * 2.0 - 1.0;
    vec4 clip = vec4(uv*2.0-1.0, z, 1.0);
    vec4 view = invProj * clip;
    return view.xyz / view.w;
}

