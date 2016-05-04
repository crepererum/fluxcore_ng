#version 140

in vec3 pointcolor;

out vec4 color;

uniform float inv_n;

void main() {
    float dcenter = distance(vec2(0.5, 0.5), gl_PointCoord);
    float fade = 1.0 - step(0.5, dcenter);
    color = vec4(pointcolor, 1.0) * fade * vec4(inv_n);
}
