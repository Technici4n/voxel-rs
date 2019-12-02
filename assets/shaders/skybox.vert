#version 450

layout(location = 0) in vec3 a_Pos;

layout(location = 0) out float pos_y;

layout(set = 0, binding = 0) uniform Temp1 { mat4 u_ViewProj; };
layout(set = 0, binding = 1) uniform Temp2 { vec3 u_Model; };


void main() {
    gl_Position = u_ViewProj * vec4(a_Pos + u_Model, 1.0);
    pos_y = a_Pos.y;
}
