#version 140

out vec4 color;

void main() {
    float dcenter = distance(vec2(0.5, 0.5), gl_PointCoord);
    float alpha = 1.0 - step(0.5, dcenter);
    color = vec4(1.0, 0.0, 0.0, alpha);
}
