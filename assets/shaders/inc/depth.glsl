// Restore view space position from depth and UV coordinates of the screen
// (in the range 0..1). Requires the inverse projection matrix.
vec3 reconstruct_view_pos(float depth, vec2 uv, mat4 invProj) {
    float z = depth * 2.0 - 1.0;
    vec4 clip = vec4(uv*2.0-1.0, z, 1.0);
    vec4 view = invProj * clip;
    return view.xyz / view.w;
}

// Linearize depth value (0..1) to view space Z coordinate
float linearize_depth(float depth, float near, float far) {
    float z_ndc = depth * 2.0 - 1.0;
    return (2.0 * near * far) / (far + near - z_ndc * (far - near));
}
