use crate::event::{EventKind, EventPayload};
use crate::tree::{UiNodeId, UiTree};

pub(crate) fn dispatch_event(tree: &mut UiTree, target: UiNodeId, payload: &EventPayload) -> bool {
    let kind = payload.kind();

    // MouseEnter/MouseLeave do not bubble
    if matches!(kind, EventKind::MouseEnter | EventKind::MouseLeave) {
        return tree.invoke_callback(target, kind, payload);
    }

    let chain = build_bubble_chain(tree, target);

    for &node_id in &chain {
        let handled = tree.invoke_callback(node_id, kind, payload);
        if handled {
            return true;
        }
    }

    false
}

fn build_bubble_chain(tree: &UiTree, target: UiNodeId) -> Vec<UiNodeId> {
    let mut chain = Vec::new();
    let mut current = Some(target);
    while let Some(id) = current {
        chain.push(id);
        current = tree.nodes.get(id).and_then(|n| n.parent);
    }
    chain
}
