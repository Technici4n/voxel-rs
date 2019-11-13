#version 330

in vec3 v_Norm;
in vec2 v_UvPos;
in vec2 v_UvSize;
in vec2 v_UvOffset;
in float occl;
in float v_LightLevel;

out vec4 ColorBuffer;

void main() {
    ColorBuffer = vec4(0.0, 0.0, 0.0, 1.0);
}
