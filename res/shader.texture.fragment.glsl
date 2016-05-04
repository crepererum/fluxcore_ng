#version 140

in vec2 v_tex_coords;

out vec4 color;

uniform float     inv_gamma;
uniform sampler2D tex;

void main() {
    vec4 tdata       = texture(tex, v_tex_coords);
    float counter    = tdata.a;
    float multiplier = pow(counter, inv_gamma) / counter;

    color = vec4(
        tdata.r * multiplier,
        tdata.g * multiplier,
        tdata.b * multiplier,
        1.0
    );
}
