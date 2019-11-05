#version 150 core


in float pos_y;
out vec4 ColorBuffer;



void main() {
    float frac = (pos_y+900.0)/1800;
    ColorBuffer = vec4(0.55, 0.84, 0.92, 1.0)*frac + 0.7*(1-frac)*vec4(0.1, 0.4, 1, 1.0);
}