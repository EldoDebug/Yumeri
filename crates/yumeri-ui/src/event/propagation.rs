use crate::event::{EventKind, EventPayload};
use crate::tree::{UiNodeId, UiTree};

pub(crate) fn dispatch_event(tree: &mut UiTree, target: UiNodeId, payload: &EventPayload) -> bool {
    let kind = payload.kind();

    // MouseEnter/MouseLeave do not bubble
    if matches!(kind, EventKind::MouseEnter | EventKind::MouseLeave) {
        return tree.invoke_callback(target, kind, payload);
    }

    // Walk the parent chain iteratively (avoids Vec allocation)
    let mut current = Some(target);
    while let Some(id) = current {
        if tree.invoke_callback(id, kind, payload) {
            return true;
        }
        current = tree.nodes.get(id).and_then(|n| n.parent);
    }

    false
}
