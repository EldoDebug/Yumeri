use super::window::WindowId;

pub struct FocusStack {
    stack: Vec<WindowId>,
}

impl FocusStack {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn raise(&mut self, id: WindowId) {
        self.stack.retain(|&x| x != id);
        self.stack.push(id);
    }

    pub fn remove(&mut self, id: WindowId) {
        self.stack.retain(|&x| x != id);
    }

    pub fn focused(&self) -> Option<WindowId> {
        self.stack.last().copied()
    }

    pub fn iter_back_to_front(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.stack.iter().copied()
    }

    pub fn iter_front_to_back(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.stack.iter().rev().copied()
    }
}
