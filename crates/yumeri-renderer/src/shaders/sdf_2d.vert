#version 450

layout(set = 0, binding = 0) readonly buffer ShapeBuffer {
    // Each instance: 16 floats
    // position(2) + size(2) + corner_radius(1) + shape_type(1) + color(4)
    // + texture_index(1) + uv_min(2) + uv_max(2) + padding(1)
    float data[];
} shapes;

layout(push_constant) uniform PushConstants {
    vec2 viewport_size;
} pc;

layout(location = 0) out vec2 v_local_pos;
layout(location = 1) flat out vec2 v_size;
layout(location = 2) flat out float v_corner_radius;
layout(location = 3) flat out uint v_shape_type;
layout(location = 4) flat out vec4 v_color;
layout(location = 5) out vec2 v_uv;
layout(location = 6) flat out int v_texture_index;

void main() {
    int instance = gl_InstanceIndex;
    int vertex = gl_VertexIndex;

    // Read instance data (16 floats per instance)
    int base = instance * 16;
    vec2 position = vec2(shapes.data[base + 0], shapes.data[base + 1]);
    vec2 size = vec2(shapes.data[base + 2], shapes.data[base + 3]);
    float corner_radius = shapes.data[base + 4];
    uint shape_type = uint(shapes.data[base + 5]);
    vec4 color = vec4(shapes.data[base + 6], shapes.data[base + 7], shapes.data[base + 8], shapes.data[base + 9]);
    float texture_index = shapes.data[base + 10];
    vec2 uv_min = vec2(shapes.data[base + 11], shapes.data[base + 12]);
    vec2 uv_max = vec2(shapes.data[base + 13], shapes.data[base + 14]);

    // Generate quad vertices (triangle strip: 4 vertices)
    // Vertex order: 0=BL, 1=BR, 2=TL, 3=TR
    // Add padding for anti-aliasing (2 pixels)
    float padding = 2.0;
    vec2 half_size = size + padding;

    vec2 offsets[4] = vec2[](
        vec2(-1.0, -1.0),
        vec2( 1.0, -1.0),
        vec2(-1.0,  1.0),
        vec2( 1.0,  1.0)
    );

    vec2 local = offsets[vertex] * half_size;
    vec2 world_pos = position + local;

    // Convert pixel coordinates to NDC: (0,0) top-left, (w,h) bottom-right -> (-1,-1) to (1,1)
    vec2 ndc = (world_pos / pc.viewport_size) * 2.0 - 1.0;

    gl_Position = vec4(ndc, 0.0, 1.0);
    v_local_pos = local;
    v_size = size;
    v_corner_radius = corner_radius;
    v_shape_type = shape_type;
    v_color = color;

    // Compute UV: map local position [-size, +size] to [uv_min, uv_max]
    vec2 safe_size = max(size, vec2(0.001));
    vec2 normalized = (local / safe_size) * 0.5 + 0.5;
    v_uv = mix(uv_min, uv_max, normalized);
    v_texture_index = int(texture_index);
}
