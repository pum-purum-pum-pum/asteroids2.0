uniform mediump vec3 fill_color;
out mediump vec4 fragColor;

void main() {
    fragColor = vec4(fill_color, 1.0);
}
