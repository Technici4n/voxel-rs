#version 450

layout(location = 0) in vec3 v_Norm;
layout(location = 1) in float occl;
layout(location = 2) in vec3 v_Rgb;

layout(location = 0) out vec4 ColorBuffer;

const vec3 SUN_DIRECTION = normalize(vec3(0, 1, 0.5));
const float SUN_FRACTION = 0.1;

void main() {
    /* VARIOUS BRIGHTNESS FACTORS */
    float normal_factor = 1.0 - SUN_FRACTION + SUN_FRACTION * dot(v_Norm, SUN_DIRECTION);
    float total_factor = occl * normal_factor;

    /* OUTPUT */
    ColorBuffer = vec4(v_Rgb * total_factor, 1.0);
}
