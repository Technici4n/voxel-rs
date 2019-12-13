#version 450

layout(location = 0) in vec3 i_position;
layout(location = 1) in vec2 i_texture_top_left;
layout(location = 2) in vec2 i_texture_size;
layout(location = 3) in vec2 i_texture_max_uv;
layout(location = 4) in vec2 i_texture_uv;
// occl at end, then face then light
layout(location = 5) in uint i_occl_and_face;
// light: 4 bits
// occl: 2 bits
// face: 3 bits

layout(set = 0, binding = 0) uniform Transform {
    mat4 u_view_proj;
};

layout(location = 0) flat out vec3 o_norm;
layout(location = 1) out float o_occl;
layout(location = 2) flat out vec2 o_texture_top_left;
layout(location = 3) flat out vec2 o_texture_size;
layout(location = 4) flat out vec2 o_texture_max_uv;
layout(location = 5) out vec2 o_texture_uv;
layout(location = 6) flat out float o_light_level;

vec3 get_normal(uint id) {
    if(id == 0u) {
        return vec3(1.0, 0.0, 0.0);
    } else if(id == 1u) {
        return vec3(-1.0, 0.0, 0.0);
    } else if(id == 2u) {
        return vec3(0.0, 1.0, 0.0);
    } else if(id == 3u) {
        return vec3(0.0, -1.0, 0.0);
    } else if(id == 4u) {
        return vec3(0.0, 0.0, 1.0);
    } else {
        return vec3(0.0, 0.0, -1.0);
    }
}

float get_occl(uint code_occl) {
    if (code_occl == 3u) {
        return 1.0;
    } else if (code_occl == 2u) {
        return 0.8;
    } else if (code_occl == 1u) {
        return 0.6;
    } else {
        return 0.4;
    }
}

void main() {

    uint light_level = (i_occl_and_face & 0x000001E0u) >> 5;
    uint occl_code = (i_occl_and_face & 0x00000018u) >> 3;
    uint face_index = (i_occl_and_face & 0x00000007u) >> 0;

    o_norm = get_normal(face_index);
    o_occl = get_occl(occl_code);
    o_texture_top_left = i_texture_top_left;
    o_texture_size = i_texture_size;
    o_texture_max_uv = i_texture_max_uv;
    o_texture_uv = i_texture_uv;
    o_light_level = float(light_level);

    gl_Position = u_view_proj * vec4(i_position, 1.0);
}
