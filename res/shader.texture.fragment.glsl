#version 140

in vec2 v_tex_coords;

out vec4 color;  // Lab + counter

uniform float     inv_gamma;
uniform sampler2D tex;

float f_inv(float t) {
    float delta = 6.0 / 29.0;
    if (t > delta) {
        return t * t * t;
    } else {
        return 3 * delta * delta * (t - 4.0 / 29.0);
    }
}

vec3 Lab2XYZ(vec3 Lab) {
    // that's the D65 white point with the 2 degrees (CIE 1931) observer
    const float Xn = 95.047;
    const float Yn = 100.0;
    const float Zn = 108.883;

    float L = Lab.x;
    float a = Lab.y;
    float b = Lab.z;

    float tmp = (L + 16.0) / 116.0;

    return vec3(
        Xn * f_inv(tmp + a / 500.0),
        Yn * f_inv(tmp),
        Zn * f_inv(tmp - b / 200.0)
    );
}

vec3 XYZ2RGB(vec3 XYZ) {
    // D65 as well
    // with sRGB
    const mat3 XYZ_to_RGB = mat3(
         3.2404542, -1.5371385, -0.4985314,
        -0.9692660,  1.8760108,  0.0415560,
         0.0556434, -0.2040259,  1.0572252
    );

    return XYZ_to_RGB * XYZ;
}

void main() {
    vec4 tdata       = texture(tex, v_tex_coords);
    float counter    = tdata.a;
    float multiplier = pow(counter, inv_gamma) / counter;

    color = vec4(
        XYZ2RGB(0.01 * Lab2XYZ(vec3(
            100.0 * tdata.x * multiplier,       // [0,1] => [0,100]
            256.0 * tdata.y / counter - 128.0,  // [0,1] => [-128,128]
            256.0 * tdata.z / counter - 128.0   // [0,1] => [-128,128]
        ))),
        1.0
    );
}
