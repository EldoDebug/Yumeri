use crate::tree::{UiNodeId, UiTree};

pub(crate) fn hit_test(tree: &UiTree, x: f32, y: f32) -> Option<UiNodeId> {
    let root = tree.root?;
    hit_test_recursive(tree, root, x, y)
}

/// Uses cached absolute bounds from last sync to avoid per-call taffy.layout() overhead.
fn hit_test_recursive(
    tree: &UiTree,
    node_id: UiNodeId,
    x: f32,
    y: f32,
) -> Option<UiNodeId> {
    let node = tree.nodes.get(node_id)?;

    if !node.style.visible {
        return None;
    }

    let [abs_x, abs_y, w, h] = node.cached_bounds?;

    // AABB test
    if x < abs_x || x > abs_x + w || y < abs_y || y > abs_y + h {
        return None;
    }

    // Check children in reverse order (last drawn = topmost)
    for &child_id in node.children.iter().rev() {
        if let Some(hit) = hit_test_recursive(tree, child_id, x, y) {
            return Some(hit);
        }
    }

    // No child hit, return this node
    Some(node_id)
}

#[allow(dead_code)]
pub(crate) fn get_node_bounds(tree: &UiTree, node_id: UiNodeId) -> Option<(f32, f32, f32, f32)> {
    let [abs_x, abs_y, w, h] = tree.nodes.get(node_id)?.cached_bounds?;
    Some((abs_x, abs_y, w, h))
}
