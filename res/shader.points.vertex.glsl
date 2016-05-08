#version 140

in vec3 position;

out vec3  pointcolor;
out float atborder;  // = bool

uniform mat4  matrix;
uniform float pointsize;
uniform float showborder;  // = bool

// http://lolengine.net/blog/2013/07/27/rgb-to-hsv-in-glsl
vec3 hsv2rgb(vec3 c) {
    vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}

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

    pointcolor = hsv2rgb(vec3(0.5 * pos_virtual.z, 1.0, 1.0)) * alpha;
}
