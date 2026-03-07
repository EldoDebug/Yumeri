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
layout(set = 2, binding = 0) uniform sampler2D s_mask;

layout(location = 0) in vec2 v_uv;
layout(location = 1) in vec4 v_clip_pos;

layout(location = 0) out vec4 out_color;

// flags.x: premultiplied alpha
// flags.y: 0=draw, 1=masked, 2=masked_inv

vec4 sample_base_color() {
    vec4 tex = texture(s_texture, v_uv);
    vec3 rgb = tex.rgb * u.multiply_color.rgb;
    if (u.flags.x > 0.5) {
        // premultiplied alpha path
        rgb = (rgb + u.screen_color.rgb * tex.a) - (rgb * u.screen_color.rgb);
        return vec4(rgb, tex.a) * u.base_color;
    } else {
        rgb = rgb + u.screen_color.rgb - (rgb * u.screen_color.rgb);
        vec4 color = vec4(rgb, tex.a) * u.base_color;
        return vec4(color.rgb * color.a, color.a);
    }
}

void main() {
    vec4 base = sample_base_color();
    float mode = u.flags.y;

    if (mode < 0.5) {
        // Normal draw (no mask)
        out_color = base;
    } else {
        // Masked draw
        vec2 clip_uv = v_clip_pos.xy / v_clip_pos.w;
        vec2 mask_uv = vec2(clip_uv.x, 1.0 - clip_uv.y);
        vec4 mask_tex = texture(s_mask, mask_uv);
        vec4 clip_mask = (vec4(1.0) - mask_tex) * u.channel_flag;
        float mask_val = clip_mask.r + clip_mask.g + clip_mask.b + clip_mask.a;

        if (mode < 1.5) {
            // Normal masked
            out_color = base * mask_val;
        } else {
            // Inverted masked
            out_color = base * (1.0 - mask_val);
        }
    }
}
