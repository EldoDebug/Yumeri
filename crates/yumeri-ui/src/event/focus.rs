use crate::tree::{UiNodeId, UiTree};

pub struct FocusState {
    focused: Option<UiNodeId>,
}

impl FocusState {
    pub fn new() -> Self {
        Self { focused: None }
    }

    pub fn focused(&self) -> Option<UiNodeId> {
        self.focused
    }

    pub fn set_focus(&mut self, node: Option<UiNodeId>) {
        self.focused = node;
    }
}

impl Default for FocusState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn collect_focusable(tree: &UiTree) -> Vec<UiNodeId> {
    let mut result = Vec::new();
    if let Some(root) = tree.root {
        collect_focusable_recursive(tree, root, &mut result);
    }
    result
}

fn collect_focusable_recursive(tree: &UiTree, node_id: UiNodeId, out: &mut Vec<UiNodeId>) {
    let node = match tree.nodes.get(node_id) {
        Some(n) => n,
        None => return,
    };

    if node.focusable && node.style.visible {
        out.push(node_id);
    }

    for &child_id in &node.children {
        collect_focusable_recursive(tree, child_id, out);
    }
}

pub(crate) fn focus_next(tree: &mut UiTree) {
    let focusable = collect_focusable(tree);
    if focusable.is_empty() {
        return;
    }

    let current = tree.focus.focused();
    let next = match current {
        Some(id) => {
            let pos = focusable.iter().position(|&fid| fid == id);
            match pos {
                Some(i) => focusable[(i + 1) % focusable.len()],
                None => focusable[0],
            }
        }
        None => focusable[0],
    };

    tree.focus.set_focus(Some(next));
}

pub(crate) fn focus_prev(tree: &mut UiTree) {
    let focusable = collect_focusable(tree);
    if focusable.is_empty() {
        return;
    }

    let current = tree.focus.focused();
    let prev = match current {
        Some(id) => {
            let pos = focusable.iter().position(|&fid| fid == id);
            match pos {
                Some(0) => *focusable.last().unwrap(),
                Some(i) => focusable[i - 1],
                None => focusable[0],
            }
        }
        None => *focusable.last().unwrap(),
    };

    tree.focus.set_focus(Some(prev));
}
