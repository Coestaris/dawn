#version 330 core
out vec4 FragColor;

in vec2 TexCoords;

uniform sampler2D color_texture;
uniform sampler2D depth_texture;


float near = 0.1;
float far  = 100.0;

float LinearizeDepth(float depth)
{
    float z = depth * 2.0 - 1.0; // back to NDC
    return (2.0 * near * far) / (far + near - z * (far - near));
}

void main()
{
    vec4 color = texture(color_texture, TexCoords);
    float depth = texture(depth_texture, TexCoords).r;

    float mist = LinearizeDepth(depth) / far;// divide by far for demonstration
    mist = clamp(mist, 0.0, 1.0);

    vec3 mistColor = vec3(0.1, 0.1, 0.1);
    vec3 finalColor = mix(color.rgb, mistColor, mist);
    FragColor = vec4(finalColor, 1.0);
}