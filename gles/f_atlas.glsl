in mediump vec2 v_tex_coords;
uniform  sampler2D tex;
out mediump vec4 fragColor;
void main() {
    mediump vec4 texture_colors = vec4(texture(tex, v_tex_coords));
	mediump float alpha = texture_colors.a;
	if (alpha < 0.0001) {
		discard;
	}
    fragColor = texture_colors;
}