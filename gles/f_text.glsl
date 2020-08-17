uniform sampler2D font_tex;

in mediump vec2 f_tex_pos;
in mediump vec4 f_color;
out mediump vec4 fragColor;

void main() {
    mediump float alpha = texture(font_tex, f_tex_pos).r;
    if (alpha <= 0.0) {
        discard;
    }
    fragColor = f_color * vec4(1.0, 1.0, 1.0, alpha);
}