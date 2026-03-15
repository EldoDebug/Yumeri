use super::shapes::{Shape, FLOATS_PER_INSTANCE};
use crate::texture::TextureId;

pub(crate) struct DrawBatch {
    shapes: Vec<Shape>,
}

impl DrawBatch {
    pub fn new() -> Self {
        Self { shapes: Vec::new() }
    }

    pub fn push(&mut self, shape: Shape) {
        self.shapes.push(shape);
    }

    pub fn instance_count(&self) -> u32 {
        self.shapes.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Write instance data directly to a mapped buffer slice without intermediate allocation.
    pub fn write_to_buffer(&self, buffer: &mut [u8], resolve: impl Fn(TextureId) -> u32) {
        let stride = FLOATS_PER_INSTANCE * size_of::<f32>();
        for (i, shape) in self.shapes.iter().enumerate() {
            let offset = i * stride;
            if offset + stride > buffer.len() {
                break;
            }
            let data = shape.to_instance_data(&resolve);
            let bytes = bytemuck::cast_slice::<f32, u8>(&data);
            buffer[offset..offset + stride].copy_from_slice(bytes);
        }
    }

    pub fn clear(&mut self) {
        self.shapes.clear();
    }
}
