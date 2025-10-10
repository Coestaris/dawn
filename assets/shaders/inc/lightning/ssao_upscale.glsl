const float DEPTH_SIGMA = 1.5;
const float NORM_POWER  = 4.0;

float upsample_ssao(vec2 uv_full) {
    ivec2 half_size = textureSize(in_halfres_ssao, 0);

    float z0 = get_depth(uv_full);
    vec3  n0 = get_normal(uv_full);

    vec2 st   = uv_full * vec2(half_size) - 0.5;
    ivec2 ij0 = ivec2(floor(st));
    vec2  frac = st - vec2(ij0);

    float acc    = 0.0;
    float weight = 0.0;

    for (int dy = 0; dy <= 1; ++dy) {
        for (int dx = 0; dx <= 1; ++dx) {
            ivec2 ij = ij0 + ivec2(dx, dy);
            ij = clamp(ij, ivec2(0), half_size - ivec2(1));
            float ao = texelFetch(in_halfres_ssao, ij, 0).r;

            vec2  sample_uv  = (vec2(ij) + 0.5) / vec2(half_size);
            float zi = get_depth(sample_uv);
            vec3  ni = get_normal(sample_uv);

            // Basic bilinear weight
            float wx = (dx == 0) ? (1.0 - frac.x) : frac.x;
            float wy = (dy == 0) ? (1.0 - frac.y) : frac.y;
            float wlin = wx * wy;

            float dz = abs(zi - z0);
            float wz = exp(-dz * DEPTH_SIGMA);

            float dn = max(dot(n0, ni), 0.0);
            float wn = dn * dn;
            wn *= wn;

            float w = wlin * wz * wn;

            acc    += ao * w;
            weight += w;
        }
    }

    return (weight > 0.0) ? (acc / weight) : 1.0;
}

float get_ssao(vec2 uv) {
    if (in_ssao_enabled != 1) return 1.0;
    return upsample_ssao(uv);
}