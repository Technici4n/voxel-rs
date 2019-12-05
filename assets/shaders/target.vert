#version 450

layout(location = 0) in vec3 a_Pos;

layout(set = 0, binding = 0) uniform Temp1 { mat4 u_ViewProj; };
layout(set = 0, binding = 1) uniform Temp2 { mat4 u_Model; };

void main() {
    gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
}
