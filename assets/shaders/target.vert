#version 330

in vec3 a_Pos;

uniform Transform {
    mat4 u_ViewProj;
    mat4 u_Model;
};

void main() {
    gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
}
