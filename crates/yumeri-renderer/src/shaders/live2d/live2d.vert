#version 450

layout(set = 0, binding = 0) uniform Uniforms {
    mat4 mvp;
    mat4 clip;
    vec4 base_color;
    vec4 multiply_color;
    vec4 screen_color;
    vec4 channel_flag;
    vec4 flags;
} u;

layout(location = 0) in vec2 in_position;
layout(location = 1) in vec2 in_uv;

layout(location = 0) out vec2 v_uv;
layout(location = 1) out vec4 v_clip_pos;

void main() {
    gl_Position = u.mvp * vec4(in_position, 0.0, 1.0);
    v_clip_pos = u.clip * vec4(in_position, 0.0, 1.0);
    v_uv = vec2(in_uv.x, 1.0 - in_uv.y);
}
