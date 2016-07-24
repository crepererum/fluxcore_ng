#version 140

in vec3 position;

out vec3  pointcolor;  // = Lab color space
out float atborder;    // = bool

uniform mat4  matrix;
uniform float pointsize;
uniform float showborder;  // = bool

void main() {
    gl_PointSize     = pointsize;
    vec4 pos_virtual = matrix * vec4(position, 1.0);
    gl_Position      = vec4(pos_virtual.xy, 0.0, 1.0);

    float alpha = 1.0;
    atborder = 0.0;
    if (showborder > 0.5) {
        float dist = 0.0;
        if (gl_Position.x > 1.0) {
            dist += gl_Position.x;
            gl_Position.x = 1.0;
        } else if (gl_Position.x < -1.0) {
            dist -= gl_Position.x;
            gl_Position.x = -1.0;
        }
        if (gl_Position.y > 1.0) {
            dist += gl_Position.y;
            gl_Position.y = 1.0;
        } else if (gl_Position.y < -1.0) {
            dist -= gl_Position.y;
            gl_Position.y= -1.0;
        }
        if (dist > 0.0) {
            gl_PointSize /= dist;
            alpha        = 1.0 / dist;
            atborder     = 1.0;
        }
    }

    float color_a_and_b = pos_virtual.z;
    pointcolor = vec3(1.0, color_a_and_b, color_a_and_b) * alpha;
}
