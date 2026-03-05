use slotmap::SlotMap;

use super::node::{Node, NodeId};
use crate::renderer::renderer2d::shapes::FLOATS_PER_INSTANCE;
use crate::texture::TextureId;

pub(crate) struct RenderList {
    instance_data: Vec<f32>,
    index_to_node: Vec<NodeId>,
}

impl RenderList {
    pub(crate) fn new() -> Self {
        Self {
            instance_data: Vec::new(),
            index_to_node: Vec::new(),
        }
    }

    pub(crate) fn instance_count(&self) -> u32 {
        self.index_to_node.len() as u32
    }

    pub(crate) fn index_to_node(&self) -> &[NodeId] {
        &self.index_to_node
    }

    pub(crate) fn rebuild(
        &mut self,
        nodes: &SlotMap<NodeId, Node>,
        roots: &[NodeId],
        resolve: &impl Fn(TextureId) -> u32,
    ) {
        self.instance_data.clear();
        self.index_to_node.clear();

        // Iterative DFS with z-sorted children.
        let mut stack: Vec<NodeId> = Vec::new();

        // Push roots in reverse z-order (so lowest z pops first)
        let mut sorted_roots: Vec<NodeId> = roots.to_vec();
        sorted_roots.sort_by_key(|&id| nodes.get(id).map_or(0, |n| n.z_index));
        for &root_id in sorted_roots.iter().rev() {
            stack.push(root_id);
        }

        let mut sorted_children: Vec<NodeId> = Vec::new();

        while let Some(node_id) = stack.pop() {
            let Some(node) = nodes.get(node_id) else {
                continue;
            };

            if !node.visible {
                continue;
            }

            if node.is_renderable() {
                self.instance_data
                    .extend_from_slice(&node.to_instance_data(resolve));
                self.index_to_node.push(node_id);
            }

            // Push children in reverse z-order so lowest-z is processed first
            sorted_children.clear();
            sorted_children.extend_from_slice(&node.children);
            sorted_children.sort_by_key(|&id| nodes.get(id).map_or(0, |n| n.z_index));
            for &child_id in sorted_children.iter().rev() {
                stack.push(child_id);
            }
        }
    }

    pub(crate) fn update_entry(&mut self, render_index: usize, data: &[f32; FLOATS_PER_INSTANCE]) {
        let offset = render_index * FLOATS_PER_INSTANCE;
        self.instance_data[offset..offset + FLOATS_PER_INSTANCE].copy_from_slice(data);
    }

    pub(crate) fn write_all(&self, buffer: &mut [u8]) {
        let bytes = bytemuck::cast_slice::<f32, u8>(&self.instance_data);
        let len = bytes.len().min(buffer.len());
        buffer[..len].copy_from_slice(&bytes[..len]);
    }

    pub(crate) fn write_ranges(&self, buffer: &mut [u8], ranges: &[(usize, usize)]) {
        let all_bytes = bytemuck::cast_slice::<f32, u8>(&self.instance_data);
        for &(byte_offset, byte_len) in ranges {
            let end = (byte_offset + byte_len).min(buffer.len()).min(all_bytes.len());
            if byte_offset < end {
                buffer[byte_offset..end].copy_from_slice(&all_bytes[byte_offset..end]);
            }
        }
    }
}
