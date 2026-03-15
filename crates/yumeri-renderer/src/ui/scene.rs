use slotmap::SlotMap;
use yumeri_font::Font;

use super::node::{DirtyFlags, Node, NodeId};
use super::render_list::RenderList;
use crate::renderer::renderer2d::shapes::{Color, ShapeType, FLOATS_PER_INSTANCE};
use crate::text::{shape_and_cache_glyphs, TextStyle};
use crate::texture::glyph_cache::GlyphCache;
use crate::texture::{Texture, TextureId};

pub(crate) enum SyncResult {
    Clean,
    Incremental(Vec<(usize, usize)>),
    FullRewrite,
}

pub struct Scene {
    nodes: SlotMap<NodeId, Node>,
    roots: Vec<NodeId>,
    dirty_nodes: Vec<NodeId>,
    tree_dirty: bool,
    render_list: RenderList,
    generation: u64,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            roots: Vec::new(),
            dirty_nodes: Vec::new(),
            tree_dirty: false,
            render_list: RenderList::new(),
            generation: 0,
        }
    }

    pub fn add(&mut self, shape_type: ShapeType) -> NodeId {
        let node = Node::new(shape_type);
        let id = self.nodes.insert(node);
        self.roots.push(id);
        self.tree_dirty = true;
        id
    }

    pub fn add_child(&mut self, parent: NodeId, shape_type: ShapeType) -> Option<NodeId> {
        if !self.nodes.contains_key(parent) {
            return None;
        }
        let mut node = Node::new(shape_type);
        node.parent = Some(parent);
        let id = self.nodes.insert(node);
        self.nodes[parent].children.push(id);
        self.tree_dirty = true;
        Some(id)
    }

    pub fn remove(&mut self, id: NodeId) -> bool {
        let Some(node) = self.nodes.remove(id) else {
            return false;
        };

        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(parent_id) {
                parent.children.retain(|&c| c != id);
            }
        } else {
            self.roots.retain(|&r| r != id);
        }

        // Iterative subtree removal
        let mut stack: Vec<NodeId> = node.children;
        while let Some(child_id) = stack.pop() {
            if let Some(child) = self.nodes.remove(child_id) {
                stack.extend(child.children);
            }
        }

        self.tree_dirty = true;
        true
    }

    pub fn set_position(&mut self, id: NodeId, position: [f32; 2]) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.position != position
        {
            node.position = position;
            node.dirty |= DirtyFlags::TRANSFORM;
            self.track_dirty(id);
            self.propagate_transform_dirty(id);
        }
    }

    pub fn set_size(&mut self, id: NodeId, size: [f32; 2]) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.size != size
        {
            node.size = size;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    pub fn set_color(&mut self, id: NodeId, color: Color) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.color != color
        {
            node.color = color;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    pub fn set_corner_radius(&mut self, id: NodeId, radius: f32) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.corner_radius != radius
        {
            node.corner_radius = radius;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    pub fn set_texture(&mut self, id: NodeId, texture: Option<Texture>) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.texture != texture
        {
            node.texture = texture;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    pub fn set_visible(&mut self, id: NodeId, visible: bool) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.visible != visible
        {
            node.visible = visible;
            self.tree_dirty = true;
        }
    }

    pub fn set_z_index(&mut self, id: NodeId, z_index: i32) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.z_index != z_index
        {
            node.z_index = z_index;
            self.tree_dirty = true;
        }
    }

    pub fn set_translate(&mut self, id: NodeId, translate: [f32; 2]) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.translate != translate
        {
            node.translate = translate;
            node.dirty |= DirtyFlags::TRANSFORM;
            self.track_dirty(id);
            self.propagate_transform_dirty(id);
        }
    }

    /// Scale is applied per-node at the GPU level and does not propagate to children.
    pub fn set_scale(&mut self, id: NodeId, scale: [f32; 2]) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.scale != scale
        {
            node.scale = scale;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    /// Rotation is applied per-node at the GPU level and does not propagate to children.
    pub fn set_rotation(&mut self, id: NodeId, rotation: f32) {
        if let Some(node) = self.nodes.get_mut(id)
            && node.rotation != rotation
        {
            node.rotation = rotation;
            node.dirty |= DirtyFlags::VISUAL;
            self.track_dirty(id);
        }
    }

    pub fn set_text(
        &mut self,
        id: NodeId,
        font: &mut Font,
        text: &str,
        style: &TextStyle,
        glyph_cache: &mut GlyphCache,
    ) {
        let fingerprint = crate::text::compute_text_fingerprint(font, text, style);
        let atlas_gen = glyph_cache.atlas_generation();

        if let Some(node) = self.nodes.get(id) {
            if node.text_fingerprint == fingerprint
                && node.text_atlas_generation == atlas_gen
                && !node.text_glyph_children.is_empty()
            {
                let has_textures = node.text_glyph_children.first()
                    .and_then(|&cid| self.nodes.get(cid))
                    .is_some_and(|c| c.texture.is_some());
                if has_textures {
                    return;
                }
            }
        }

        let (layout_glyphs, atlas_id, _) = shape_and_cache_glyphs(font, text, style, glyph_cache);

        // Glyph positions from text layout are relative to top-left,
        // but the parent node uses center+half-extents coordinates.
        // Offset by [-half_w, -half_h] to convert to parent-local coords.
        let parent_size = self.nodes.get(id).map(|n| n.size).unwrap_or([0.0, 0.0]);
        let origin = [-parent_size[0], -parent_size[1]];

        let existing_children: Vec<NodeId> = self
            .nodes
            .get(id)
            .map(|n| n.text_glyph_children.clone())
            .unwrap_or_default();
        let old_count = existing_children.len();
        let new_count = layout_glyphs.len();
        let reuse_count = old_count.min(new_count);
        let structure_changed = old_count != new_count;

        let mut glyph_child_ids = Vec::with_capacity(new_count);

        for (i, lg) in layout_glyphs.iter().enumerate() {
            let texture = atlas_id.map(|tid| crate::texture::Texture { id: tid, uv_rect: lg.cached.uv });
            let rect = lg.to_rect(origin, style.color, texture);

            if i < reuse_count {
                // Reuse existing glyph node — update only changed properties
                let child_id = existing_children[i];
                if let Some(child) = self.nodes.get_mut(child_id) {
                    let mut dirty = DirtyFlags::empty();
                    if child.position != rect.position {
                        child.position = rect.position;
                        dirty |= DirtyFlags::TRANSFORM;
                    }
                    if child.size != rect.size || child.color != rect.color || child.texture != rect.texture {
                        child.size = rect.size;
                        child.color = rect.color;
                        child.texture = rect.texture;
                        dirty |= DirtyFlags::VISUAL;
                    }
                    if !dirty.is_empty() {
                        child.dirty |= dirty;
                        self.dirty_nodes.push(child_id);
                    }
                }
                glyph_child_ids.push(child_id);
            } else {
                // Need a new glyph node
                let Some(child_id) = self.add_child(id, ShapeType::Rect) else {
                    continue;
                };
                if let Some(child) = self.nodes.get_mut(child_id) {
                    child.position = rect.position;
                    child.size = rect.size;
                    child.color = rect.color;
                    child.texture = rect.texture;
                }
                glyph_child_ids.push(child_id);
            }
        }

        // Remove excess old glyph children
        for &child_id in &existing_children[reuse_count..] {
            self.remove(child_id);
        }

        if let Some(node) = self.nodes.get_mut(id) {
            node.text_glyph_children = glyph_child_ids;
            node.text_fingerprint = fingerprint;
            node.text_atlas_generation = atlas_gen;
        }

        if structure_changed {
            self.tree_dirty = true;
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_dirty(&self) -> bool {
        self.tree_dirty || !self.dirty_nodes.is_empty()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn render_list(&self) -> &RenderList {
        &self.render_list
    }

    fn track_dirty(&mut self, id: NodeId) {
        self.dirty_nodes.push(id);
    }

    fn propagate_transform_dirty(&mut self, root: NodeId) {
        // Iterative BFS to mark all descendants as TRANSFORM-dirty
        let mut stack = Vec::new();
        if let Some(node) = self.nodes.get(root) {
            stack.extend_from_slice(&node.children);
        }
        while let Some(id) = stack.pop() {
            if let Some(node) = self.nodes.get_mut(id) {
                node.dirty |= DirtyFlags::TRANSFORM;
                self.dirty_nodes.push(id);
            }
            if let Some(node) = self.nodes.get(id) {
                stack.extend_from_slice(&node.children);
            }
        }
    }

    fn recompute_world_positions(&mut self) {
        // Iterative DFS with parent world position on the stack
        let mut stack: Vec<(NodeId, [f32; 2])> = self
            .roots
            .iter()
            .map(|&id| (id, [0.0, 0.0]))
            .collect();

        while let Some((id, parent_pos)) = stack.pop() {
            let Some(node) = self.nodes.get_mut(id) else {
                continue;
            };
            node.world_position = [
                parent_pos[0] + node.position[0] + node.translate[0],
                parent_pos[1] + node.position[1] + node.translate[1],
            ];
            let wp = node.world_position;
            let children = &node.children;
            for &child_id in children {
                stack.push((child_id, wp));
            }
        }
    }

    fn recompute_world_position_subtree(&mut self, id: NodeId) {
        let parent_world_pos = self
            .nodes
            .get(id)
            .and_then(|n| n.parent)
            .and_then(|pid| self.nodes.get(pid))
            .map(|p| p.world_position)
            .unwrap_or([0.0, 0.0]);

        let mut stack = vec![(id, parent_world_pos)];
        while let Some((nid, parent_pos)) = stack.pop() {
            let Some(node) = self.nodes.get_mut(nid) else {
                continue;
            };
            node.world_position = [
                parent_pos[0] + node.position[0] + node.translate[0],
                parent_pos[1] + node.position[1] + node.translate[1],
            ];
            let wp = node.world_position;
            let children = &node.children;
            for &child_id in children {
                stack.push((child_id, wp));
            }
        }
    }

    pub(crate) fn sync(
        &mut self,
        resolve: impl Fn(TextureId) -> u32,
    ) -> SyncResult {
        if !self.is_dirty() {
            return SyncResult::Clean;
        }

        if self.tree_dirty {
            self.recompute_world_positions();
            self.render_list.rebuild(&self.nodes, &self.roots, &resolve);

            // Clear all render indices so invisible/removed nodes don't retain stale values
            for (_, node) in &mut self.nodes {
                node.render_index = None;
            }

            for (i, &node_id) in self.render_list.index_to_node().iter().enumerate() {
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.render_index = Some(i as u32);
                }
            }

            // Clear dirty flags only on tracked dirty nodes
            for &id in &self.dirty_nodes {
                if let Some(node) = self.nodes.get_mut(id) {
                    node.dirty = DirtyFlags::empty();
                }
            }
            self.dirty_nodes.clear();
            self.tree_dirty = false;
            self.generation += 1;
            return SyncResult::FullRewrite;
        }

        // Only TRANSFORM and/or VISUAL changes -- use tracked dirty_nodes
        let mut dirty_nodes = std::mem::take(&mut self.dirty_nodes);
        dirty_nodes.sort_unstable();
        dirty_nodes.dedup();

        // Recompute world positions only for subtree roots (parent not TRANSFORM-dirty).
        // Safety: propagate_transform_dirty adds ALL descendants to dirty_nodes,
        // so a dirty parent is guaranteed to be processed as a subtree root.
        for &id in &dirty_nodes {
            if let Some(node) = self.nodes.get(id)
                && node.dirty.contains(DirtyFlags::TRANSFORM)
            {
                let parent_is_dirty = node
                    .parent
                    .and_then(|pid| self.nodes.get(pid))
                    .is_some_and(|p| p.dirty.contains(DirtyFlags::TRANSFORM));
                if !parent_is_dirty {
                    self.recompute_world_position_subtree(id);
                }
            }
        }

        // Collect updated instance data and byte ranges
        let stride = FLOATS_PER_INSTANCE * size_of::<f32>();
        let updates: Vec<(usize, [f32; FLOATS_PER_INSTANCE])> = dirty_nodes
            .iter()
            .filter_map(|&id| {
                let node = self.nodes.get(id)?;
                node.render_index
                    .map(|idx| (idx as usize, node.to_instance_data(&resolve)))
            })
            .collect();

        let mut ranges = Vec::with_capacity(updates.len());
        for (render_idx, data) in &updates {
            self.render_list.update_entry(*render_idx, data);
            ranges.push((render_idx * stride, stride));
        }

        for &id in &dirty_nodes {
            if let Some(node) = self.nodes.get_mut(id) {
                node.dirty = DirtyFlags::empty();
            }
        }
        self.generation += 1;

        if ranges.is_empty() {
            SyncResult::Clean
        } else if ranges.len() > self.render_list.instance_count() as usize / 2 {
            SyncResult::FullRewrite
        } else {
            SyncResult::Incremental(ranges)
        }
    }
}
