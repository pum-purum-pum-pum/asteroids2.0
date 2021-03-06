
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

in vec2 position;

uniform mediump mat4 perspective;
uniform mediump mat4 view;
uniform mediump mat4 model;

void main() {
    gl_Position = INVERT_Y_AXIS * perspective * view * model * vec4(position, 0.0, 1.0);
}
