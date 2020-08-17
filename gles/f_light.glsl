out mediump vec4 fragColor;
uniform mediump vec3 color;

void main() {
    fragColor = vec4(color, 1.0);
}
