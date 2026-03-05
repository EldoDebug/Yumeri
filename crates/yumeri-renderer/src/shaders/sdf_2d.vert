#version 450

layout(set = 0, binding = 0) readonly buffer ShapeBuffer {
    // Each instance: position(2) + size(2) + corner_radius(1) + shape_type(1 as float) + color(4)
    // Total: 10 floats per instance
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

void main() {
    int instance = gl_InstanceIndex;
    int vertex = gl_VertexIndex;

    // Read instance data (10 floats per instance)
    int base = instance * 10;
    vec2 position = vec2(shapes.data[base + 0], shapes.data[base + 1]);
    vec2 size = vec2(shapes.data[base + 2], shapes.data[base + 3]);
    float corner_radius = shapes.data[base + 4];
    uint shape_type = uint(shapes.data[base + 5]);
    vec4 color = vec4(shapes.data[base + 6], shapes.data[base + 7], shapes.data[base + 8], shapes.data[base + 9]);

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
}
