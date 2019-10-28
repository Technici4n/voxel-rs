#version 330

in vec4 v_Color;

out vec4 ColorBuffer;

void main() {
    ColorBuffer = v_Color;
}
