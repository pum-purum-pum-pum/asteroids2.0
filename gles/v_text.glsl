// #extension GL_EXT_gpu_shader4 : require
const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

uniform mat4 transform;
uniform mat4 view;

in vec3 left_top;
in vec2 right_bottom;
in vec2 tex_left_top;
in vec2 tex_right_bottom;
in vec4 color;

out mediump vec2 f_tex_pos;
out mediump vec4 f_color;

// generate positional data based on vertex ID
void main() {
    vec2 pos = vec2(0.0);
    // mediump float left = left_top.x;
    // mediump float right = right_bottom.x;
    // mediump float top = left_top.y;
    // mediump float bottom = right_bottom.y;

    mediump float x_mid = (left_top.x + right_bottom.x) / 2.0;
    mediump float y_mid = (right_bottom.y + left_top.y) / 2.0;
    mediump float left = x_mid + (left_top.x - x_mid) / 1.0;
    mediump float right = x_mid + (right_bottom.x - x_mid) / 1.0;
    mediump float top = y_mid + (left_top.y - y_mid) / 1.0;
    mediump float bottom = y_mid + (right_bottom.y - y_mid) / 1.0;

    if (gl_VertexID == 0) {
        pos = vec2(left, top);
        f_tex_pos = tex_left_top;
    };
    if (gl_VertexID == 1){
        pos = vec2(right, top);
        f_tex_pos = vec2(tex_right_bottom.x, tex_left_top.y);
    };
    if (gl_VertexID == 2) {
        pos = vec2(left, bottom);
        f_tex_pos = vec2(tex_left_top.x, tex_right_bottom.y);
    };
    if (gl_VertexID == 3) {
        pos = vec2(right, bottom);
        f_tex_pos = tex_right_bottom;
    };

    f_color = color;
    gl_Position = INVERT_Y_AXIS * transform * view * vec4(pos, left_top.z, 1.0);
}