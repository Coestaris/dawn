#include "inc/prelude.glsl"
#include "inc/ubo_camera.glsl"
#include "inc/depth.glsl"
#include "inc/normal.glsl"

// R16F. Linear depth
layout (location = 0) out float out_halres_depth;
// RG8_SNORM. Octo encoded normal, view space
layout (location = 1) out vec2 out_halres_normal;

// DEPTH24. OpenGL default depth format
uniform sampler2D in_depth;
// RG8_SNORM. Octo encoded normal, view space
uniform sampler2D in_normal;

void main() {
    ivec2 full_sz = textureSize(in_depth, 0);
    ivec2 half_sz = full_sz / 2;

    ivec2 h = ivec2(gl_FragCoord.xy);
    ivec2 f0 = h * 2;

    float d0 = texelFetch(in_depth,  f0 + ivec2(0,0), 0).r;
    float d1 = texelFetch(in_depth,  f0 + ivec2(1,0), 0).r;
    float d2 = texelFetch(in_depth,  f0 + ivec2(0,1), 0).r;
    float d3 = texelFetch(in_depth,  f0 + ivec2(1,1), 0).r;

    float d_max = d0; ivec2 off = ivec2(0,0);
    if (d1 > d_max) { d_max = d1; off = ivec2(1,0); }
    if (d2 > d_max) { d_max = d2; off = ivec2(0,1); }
    if (d3 > d_max) { d_max = d3; off = ivec2(1,1); }

    // TODO: Implement some kind of depth guided blending?
    //       E.g., if d_max is much larger than the others,
    //       we could try to pick the second largest, etc.
    out_halres_depth  = linearize_depth(d_max, in_clip_planes.x, in_clip_planes.y);
    out_halres_normal = texelFetch(in_normal, f0 + off, 0).rg;
}
