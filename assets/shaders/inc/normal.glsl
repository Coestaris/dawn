// Decode a normal from an octahedral encoded vector
vec3 decode_oct(vec2 e) {
    vec3 v = vec3(e*2.0-1.0, 1.0 - abs(e.x*2.0-1.0) - abs(e.y*2.0-1.0));
    float t = clamp(-v.z, 0.0, 1.0);
    v.x += v.x >= 0.0 ? -t : t;
    v.y += v.y >= 0.0 ? -t : t;
    return normalize(v);
}

// Encode a normal into an octahedral encoded vector
vec2 encode_oct(vec3 n) {
    n /= (abs(n.x) + abs(n.y) + abs(n.z));
    vec2 enc = n.xy;
    if (n.z < 0.0) {
        enc = (1.0 - vec2(abs(enc.y), abs(enc.x))) * vec2(sign(enc.x), sign(enc.y));
    }
    return enc * 0.5 + 0.5;
}
