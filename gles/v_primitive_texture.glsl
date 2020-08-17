
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

in vec2 tex_coords;
in vec2 position;
out mediump vec2 v_tex_coords;

uniform mediump mat4 projection;
uniform mediump mat4 view;
uniform mediump mat4 model;
// uniform mediump float size;
uniform mediump vec2 dim_scales;
uniform vec2 offset;
uniform vec2 fraction_wh;

void main() {
    v_tex_coords = offset + vec2(tex_coords.x * fraction_wh.x, tex_coords.y * fraction_wh.y);
	vec2 position_scaled = dim_scales * (1.0 + position) / 2.0;
    gl_Position = INVERT_Y_AXIS * projection * view * model * vec4(position_scaled, -1.0, 1.0);
}