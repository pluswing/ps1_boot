#version 330 core

in vec3 color;
out vec4 flag_color;

void main() {
  flag_color = vec4(color, 1.0);
}
