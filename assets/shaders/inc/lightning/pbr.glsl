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

vec3 shade_point(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic, float ao) {
    vec3 light_position = get_light_point_position(L);
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
    vec3 light_color = get_light_color(L);
    vec3 Lc = light_color * get_light_intensity(L);

    // Attenuation
    float radius = get_light_point_radius(L);
    bool linear = get_light_point_falloff_linear(L);
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
    vec3 diff = brdf_lambert(albedo, metallic) * ao;
    return (diff + spec) * Lc * (NoL * atten);
}

vec3 shade_sun(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic, float ao) {
    N = normalize(N);
    V = normalize(V);

    vec3  Ldir = -get_light_sun_direction(L);
    float NoL  = max(dot(N, Ldir), 0.0);
    if (NoL <= 0.0) {
    }

    vec3 light_color = get_light_color(L);
    vec3 Lc = light_color * get_light_intensity(L);

    float a = max(rough * rough, 1e-4);
    vec3  H = normalize(V + Ldir);
    float NoV = max(dot(N, V), 1e-4);
    float NoH = max(dot(N, H), 1e-4);
    float HoV = max(dot(H, V), 1e-4);

    vec3 F0  = mix(vec3(0.04), albedo, metallic);

    float D   = D_GGX(NoH, a);
    float Vg  = V_SmithGGXCorrelated(NoV, NoL, a);
    vec3  F   = F_Schlick(F0, HoV);

    vec3  diff = brdf_lambert(albedo, metallic);
    vec3  spec = (D * Vg) * F;

    vec3 Lo_direct = (NoL > 0.0) ? (diff + spec) * Lc * NoL : vec3(0.0);

    float ambSun = get_light_sun_ambient(L);
    float NoUp = clamp(dot(N, normalize(ENV_UP)) * 0.5 + 0.5, 0.0, 1.0);
    vec3 hemiIrradiance = mix(in_ground_color, in_sky_color, NoUp) * ambSun * in_diffuse_scale;
    vec3 ambientDiffuse = albedo * hemiIrradiance * (1.0 - metallic) * ao;

    float avgF0 = clamp((F0.x + F0.y + F0.z) * (1.0 / 3.0), 0.0, 1.0);
    ambientDiffuse *= (1.0 - 0.25 * avgF0);

    vec3 F_amb = F_Schlick(F0, NoV);
    float roughAtten = mix(1.0, 0.5, clamp(rough, 0.0, 1.0));
    vec3 specAmb = F_amb * ambSun * in_specular_scale * roughAtten * ao;

    return Lo_direct + ambientDiffuse + specAmb;
}

vec3 shade_spot(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic, float ao) {
    return vec3(0.0, 0.2, 0.0); // Placeholder
}

vec3 shade_area_rect(PackedLight L, vec3 P, vec3 N, vec3 V, vec3 albedo, float rough, float metallic, float ao) {
    return vec3(0.0, 1.0, 0.0); // Placeholder
}