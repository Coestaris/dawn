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