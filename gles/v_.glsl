
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

in vec2 tex_coords;
in vec2 position;
out mediump vec2 v_tex_coords;

uniform mediump mat4 perspective;
uniform mediump mat4 view;
uniform mediump mat4 model;
uniform mediump float scale;
uniform mediump vec2 dim_scales;

mediump vec2 position_scaled;

void main() {
    v_tex_coords = tex_coords;
    position_scaled = scale * dim_scales * position;
    gl_Position = INVERT_Y_AXIS * perspective * view * model * vec4(position_scaled, 0.0, 1.0);
}
