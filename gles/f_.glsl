in mediump vec2 v_tex_coords;
uniform  sampler2D tex;
out mediump vec4 fragColor;
mediump float alpha;

void main() {
    mediump vec4 texture_colors = vec4(texture(tex, v_tex_coords));
	alpha = texture_colors.a;
	if (alpha < 0.0001) {
		discard;
	}
    // if (texture_colors.r - texture_colors.g < 0.1 && texture_colors.r > 0.4) {
    // 	texture_colors.r = 1.0;
    // 	texture_colors.g = 0.0;
    // 	texture_colors.b = 0.0;
    // }
    fragColor = texture_colors;
}
