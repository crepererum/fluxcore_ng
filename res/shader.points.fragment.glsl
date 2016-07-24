#version 140

in vec3  pointcolor;  // Lab color space
in float atborder;

out vec4 color;       // Lab + counter

uniform float inv_n;

void main() {
    vec2  delta    = vec2(0.5, 0.5) - gl_PointCoord;
    float dcenter2 = dot(delta, delta);

    float fade = 1.0 - step(0.25, dcenter2);

    if (atborder > 0.5) {
        fade *= step(0.15, dcenter2);
    }

    if (fade < 0.000001) {
        discard;
    }
    color = vec4(pointcolor, 1.0) * fade * vec4(inv_n);
}
