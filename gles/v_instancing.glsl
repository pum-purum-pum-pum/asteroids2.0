
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

in vec2 position;
in vec3 world_position;

uniform mediump mat4 perspective;
uniform mediump mat4 view;
uniform mediump mat4 model;

mediump vec3 position_moved;

void main() {
    position_moved = world_position + vec3(position, 0.0);
    gl_Position = INVERT_Y_AXIS * perspective * view * model * vec4(position_moved, 1.0);
}
