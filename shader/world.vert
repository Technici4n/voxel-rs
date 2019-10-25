#version 150 core

in vec3 a_Pos;
in uint a_Norm;

uniform Transform {
    mat4 u_ViewProj;
    mat4 u_Model;
};

out vec3 v_Norm;

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

void main() {
    gl_Position = u_ViewProj * u_Model * vec4(a_Pos, 1.0);
    v_Norm = get_normal(a_Norm);
}