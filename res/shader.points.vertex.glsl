#version 140

in vec2 position;

uniform float pointsize;

void main() {
    gl_PointSize = pointsize;
    gl_Position  = vec4(position, 0.0, 1.0);
}
