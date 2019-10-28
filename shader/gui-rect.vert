#version 330

in vec3 a_Pos;
in vec4 a_Color;

uniform Transform {
    mat4 u_Transform;
    bool u_Debug;
};

out vec4 v_Color;

void main() {
    gl_Position = u_Transform * vec4(a_Pos, 1.0);

    v_Color = a_Color;
}
