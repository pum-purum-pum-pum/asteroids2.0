
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

in vec2 position;
in vec2 tex_coords;

in vec2 offset;
in vec2 fraction_wh;
in vec2 dim_scales;
in float transparency;
in vec4 color;

in vec3 world_position;
in float angle;
in float scale;

out mediump vec2 v_tex_coords;
out mediump float alpha;
out mediump vec4 color_blend;

uniform mediump mat4 perspective;
uniform mediump mat4 view;
// uniform mediump mat4 model;
// uniform mediump vec2 dim_scales;

mediump vec2 position_scaled;

void main() {
	alpha = transparency;
	color_blend = color;
	mat4 model = mat4(
	    vec4(cos(angle), sin(angle), 0.0, 0.0),
	    vec4(-sin(angle), cos(angle), 0.0, 0.0),
	    vec4(0.0, 0.0, 1.0, 0.0),
	    vec4(world_position.x, world_position.y, world_position.z, 1.0)
	);

    v_tex_coords = offset + 
    			vec2(tex_coords.x * fraction_wh.x, tex_coords.y * fraction_wh.y);
    position_scaled = scale * dim_scales * position;
    gl_Position = INVERT_Y_AXIS * perspective * view * model * 
    				vec4(position_scaled, 0.0, 1.0);
}
