#version 450

layout(location = 0) flat in vec3 i_norm;
layout(location = 1) in float i_occl;
layout(location = 2) flat in vec2 i_texture_top_left;
layout(location = 3) flat in vec2 i_texture_size;
layout(location = 4) flat in vec2 i_texture_max_uv;
layout(location = 5) in vec2 i_texture_uv;
layout(location = 6) flat in float i_light_level;

layout(location = 0) out vec4 o_color;

layout(set = 0, binding = 1) uniform sampler u_sampler;
layout(set = 0, binding = 2) uniform texture2D u_texture_atlas;

const vec3 SUN_DIRECTION = normalize(vec3(0, 1, 0.5));
const float SUN_FRACTION = 0.1;
const vec2 EPSILON = vec2(1e-7, 1e-7);

void main() {
    /* TEXTURE ACCESS */
    // avoid going out of bounds when multisampling is enabled
    vec2 corrected_uv = clamp(i_texture_uv, EPSILON, i_texture_max_uv - EPSILON);
    // compute the texture gradients before texture wrapping
    vec2 x_derivative = dFdx(corrected_uv);
    vec2 y_derivative = dFdy(corrected_uv);
    // wrap texture
    vec2 actual_uv = i_texture_top_left + mod(corrected_uv, i_texture_size);
    // get texture value
    vec4 tex_color = textureGrad(sampler2D(u_texture_atlas, u_sampler), actual_uv, x_derivative, y_derivative);

    /* VARIOUS BRIGHTNESS FACTORS */
    float light_factor = pow(0.8, 15.0 - i_light_level);
    float normal_factor = 1.0 - SUN_FRACTION + SUN_FRACTION * dot(i_norm, SUN_DIRECTION);
    float total_factor = light_factor * i_occl * normal_factor;

    /* OUTPUT */
    o_color = vec4(total_factor, total_factor, total_factor, 1.0) * tex_color;
}
