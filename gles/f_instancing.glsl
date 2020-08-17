uniform mediump float transparency;
mediump float alpha = 0.5;

out mediump vec4 fragColor;

void main() {
    fragColor =  vec4(1.0, 1.0, 1.0, alpha + (1.0 - alpha) * transparency);
}
