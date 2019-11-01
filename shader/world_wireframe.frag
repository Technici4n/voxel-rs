#version 150 core

in vec3 v_Norm;
in float occl;

out vec4 ColorBuffer;

void main() {
    ColorBuffer = vec4(0.0, 0.0, 0.0, 1.0);
}
