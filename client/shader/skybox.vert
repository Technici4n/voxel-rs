#version 150 core

in vec3 a_Pos;

out float pos_y;

uniform Transform {
    mat4 u_ViewProj;
    mat4 u_Model;
};


void main() {
    gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
    pos_y = a_Pos.y;
}
