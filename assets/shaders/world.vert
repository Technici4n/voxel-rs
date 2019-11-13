#version 330

in vec3 a_Pos;
in vec2 a_UvPos;
in vec2 a_UvOffset;
in vec2 a_UvSize;
in uint a_Norm;

uniform Transform {
    mat4 u_ViewProj;
    mat4 u_Model;
};

out vec3 v_Norm;
out float occl;
out vec2 v_UvPos;
out vec2 v_UvOffset;
out vec2 v_UvSize;
out float v_LightLevel;

uint get_light_level(uint norm) {
    return norm >> 5;
}

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

    uint norm = a_Norm;

    uint light_level = norm >> 5;
    norm -= light_level << 5;
    uint code_occl = norm >> 3;
    norm -= code_occl << 3;
    uint code_normal = norm;

    v_Norm = get_normal(code_normal);

    if(code_occl == 3u){
      occl = 1.0;
    }else if(code_occl == 2u){
      occl = 0.65;
    }else if(code_occl == 1u){
      occl = 0.4;
    }else{
      occl = 0.3;
    }

    v_UvPos = a_UvPos;
    v_UvOffset = a_UvOffset;
    v_UvSize = a_UvSize;
    v_LightLevel = float(light_level);
}
