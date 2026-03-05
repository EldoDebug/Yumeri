#version 450

layout(location = 0) in vec2 v_local_pos;
layout(location = 1) flat in vec2 v_size;
layout(location = 2) flat in float v_corner_radius;
layout(location = 3) flat in uint v_shape_type;
layout(location = 4) flat in vec4 v_color;

layout(location = 0) out vec4 out_color;

float sdf_rect(vec2 p, vec2 half_size) {
    vec2 d = abs(p) - half_size;
    return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
}

float sdf_rounded_rect(vec2 p, vec2 half_size, float radius) {
    vec2 d = abs(p) - half_size + radius;
    return length(max(d, 0.0)) - radius;
}

float sdf_circle(vec2 p, float radius) {
    return length(p) - radius;
}

void main() {
    float dist;

    if (v_shape_type == 0u) {
        // Rect
        dist = sdf_rect(v_local_pos, v_size);
    } else if (v_shape_type == 1u) {
        // RoundedRect
        dist = sdf_rounded_rect(v_local_pos, v_size, v_corner_radius);
    } else {
        // Circle (shape_type == 2)
        dist = sdf_circle(v_local_pos, v_size.x); // size.x = radius for circle
    }

    // Anti-aliasing: smooth edge over ~1.5 pixels
    float aa_width = fwidth(dist) * 1.0;
    float alpha = 1.0 - smoothstep(-aa_width, aa_width, dist);

    if (alpha < 0.001) {
        discard;
    }

    out_color = vec4(v_color.rgb, v_color.a * alpha);
}
