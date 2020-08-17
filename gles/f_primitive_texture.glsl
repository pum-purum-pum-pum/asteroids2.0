in mediump vec2 v_tex_coords;
out mediump vec4 fragColor;

uniform sampler2D tex;
void main() {
    fragColor = texture(tex, v_tex_coords);
}