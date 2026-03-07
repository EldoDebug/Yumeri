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

layout(set = 1, binding = 0) uniform sampler2D s_texture;

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec4 v_my_pos;

layout(location = 0) out vec4 out_color;

void main() {
    vec2 my = v_my_pos.xy / v_my_pos.w;
    float inside =
        step(u.base_color.x, my.x) *
        step(u.base_color.y, my.y) *
        step(my.x, u.base_color.z) *
        step(my.y, u.base_color.w);
    float a = texture(s_texture, v_uv).a;
    out_color = u.channel_flag * a * inside;
}
