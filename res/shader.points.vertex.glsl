#version 140

in vec2 position;

uniform mat4  matrix;
uniform float pointsize;

void main() {
    gl_PointSize = pointsize;
    gl_Position  = matrix * vec4(position, 0.0, 1.0);
}
