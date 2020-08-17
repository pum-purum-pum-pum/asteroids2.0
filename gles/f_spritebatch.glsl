in mediump vec2 v_tex_coords;
in mediump float alpha;
in mediump vec4 color_blend;
uniform  sampler2D tex;
out mediump vec4 fragColor;
void main() {
    mediump vec4 texture_colors = vec4(texture(tex, v_tex_coords));
    texture_colors.a = texture_colors.a * alpha;
    texture_colors.xyz = 
    	color_blend.a * color_blend.xyz + 
    	(1.0 - color_blend.a) * texture_colors.xyz;
	mediump float a = texture_colors.a;
	if (a < 0.0001) {
		discard;
	}
    fragColor = texture_colors;
}