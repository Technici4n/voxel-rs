#version 450

layout(set = 0, binding = 0) uniform Transform {
    mat4 u_transform;
};

layout(location = 0) in vec3 i_position;
layout(location = 1) in vec4 i_color;

layout(location = 0) out vec4 o_color;

void main() {
    gl_Position = u_transform * vec4(i_position, 1.0);

    o_color = i_color;
}
