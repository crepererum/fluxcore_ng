#version 140

in vec3 pointcolor;

out vec4 color;

uniform float inv_n;

void main() {
    vec2  delta    = vec2(0.5, 0.5) - gl_PointCoord;
    float dcenter2 = dot(delta, delta);

    // discard with some offset (for smoothing)
    if (dcenter2 > 0.26) {
        discard;
    }

    float fade = 1.0 - step(0.25, dcenter2);
    color = vec4(pointcolor, 1.0) * fade * vec4(inv_n);
}
