#version 450


layout(location = 0) in vec3 pos;
layout(location = 0) out vec4 ColorBuffer;


float dist_sphere(vec3 v1, vec3 v2){
    float cos_angle = dot(v1, v2);
    float angle = acos(cos_angle);
    return min(angle, 2.0*3.1415926535 - angle);
}


vec3 getSky(vec3 pos, vec3 sun_pos)
{
    float y_lim = clamp(pos.y, 0.0, 1.0) - 5*clamp(pos.y, -0.2, 0.0);
    float atmosphere = pow(1.0-y_lim, 1.4);
    vec3 skyColor = vec3(0.2,0.4,0.8);

    float scatter = pow(1.0 - dist_sphere(pos, sun_pos)/(3.1415926535), 1.0 / 30.0);
    scatter = 1.0 - clamp(scatter,0.8,1.0);

    vec3 scatterColor = mix(vec3(1.0),vec3(1.0,0.3,0.0) * 1.5,scatter);
    return mix(skyColor,vec3(scatterColor), atmosphere / 1.3);

}

vec3 getSun(vec3 pos, vec3 sun_pos){
    float sun = 1.0 - dist_sphere(pos, sun_pos);
    sun = clamp(sun,0.0,1.0);

    float glow = sun;
    glow = clamp(glow,0.0,1.0);

    sun = pow(sun,100.0);
    sun *= 100.0;
    sun = clamp(sun,0.0,1.0);

    float y_lim = clamp(pos.y, 0.0, 1.0) - 5*clamp(pos.y, -0.2, 0.0);

    glow = pow(glow,6.0) * 1.0;
    glow = pow(glow,(y_lim));
    glow = clamp(glow,0.0,1.0);

    sun *= pow(dot(y_lim, y_lim), 1.0 / 1.65);

    glow *= pow(dot(y_lim, y_lim), 1.0 / 2.0);

    sun += glow;

    vec3 sunColor = vec3(1.0,0.6,0.05) * sun;

    return vec3(sunColor);
}

void main() {
    vec3 pos_norm = normalize(pos);
    vec3 sun_pos = normalize(vec3(1.0, 0.75, 1.0));
    vec3 sky = getSky(pos_norm, sun_pos);
    vec3 sun = getSun(pos_norm, sun_pos);

    ColorBuffer = vec4(sky + sun,1.0);


}