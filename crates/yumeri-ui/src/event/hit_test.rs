use crate::tree::{UiNodeId, UiTree};

pub(crate) fn hit_test(tree: &UiTree, x: f32, y: f32) -> Option<UiNodeId> {
    let root = tree.root?;
    hit_test_recursive(tree, root, x, y, 0.0, 0.0)
}

fn hit_test_recursive(
    tree: &UiTree,
    node_id: UiNodeId,
    x: f32,
    y: f32,
    parent_x: f32,
    parent_y: f32,
) -> Option<UiNodeId> {
    let node = tree.nodes.get(node_id)?;

    if !node.style.visible {
        return None;
    }

    let layout = tree.taffy.layout(node.taffy_node).ok()?;
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    // AABB test
    if x < abs_x || x > abs_x + w || y < abs_y || y > abs_y + h {
        return None;
    }

    // Check children in reverse order (last drawn = topmost)
    let scroll_offset = node.props.scroll_offset.unwrap_or([0.0, 0.0]);
    let child_x = abs_x + scroll_offset[0];
    let child_y = abs_y + scroll_offset[1];

    for &child_id in node.children.iter().rev() {
        if let Some(hit) = hit_test_recursive(tree, child_id, x, y, child_x, child_y) {
            return Some(hit);
        }
    }

    // No child hit, return this node
    Some(node_id)
}

#[allow(dead_code)]
pub(crate) fn get_node_bounds(tree: &UiTree, node_id: UiNodeId) -> Option<(f32, f32, f32, f32)> {
    let (abs_x, abs_y) = compute_abs_pos(tree, node_id)?;
    let node = tree.nodes.get(node_id)?;
    let layout = tree.taffy.layout(node.taffy_node).ok()?;
    Some((abs_x, abs_y, layout.size.width, layout.size.height))
}

#[allow(dead_code)]
fn compute_abs_pos(tree: &UiTree, node_id: UiNodeId) -> Option<(f32, f32)> {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut current = Some(node_id);
    while let Some(id) = current {
        let node = tree.nodes.get(id)?;
        let layout = tree.taffy.layout(node.taffy_node).ok()?;
        x += layout.location.x;
        y += layout.location.y;
        current = node.parent;
    }
    Some((x, y))
}
