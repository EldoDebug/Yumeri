use std::collections::HashSet;

use crate::event::{ButtonState, InputEvent, Modifiers, PointerEventKind};
use crate::key::KeyCode;
use crate::mouse::MouseButton;

#[derive(Debug, Clone)]
pub struct InputState {
    pressed_keys: HashSet<KeyCode>,
    pressed_buttons: HashSet<MouseButton>,
    modifiers: Modifiers,
    pointer_position: (f64, f64),
}

impl InputState {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            pressed_buttons: HashSet::new(),
            modifiers: Modifiers::NONE,
            pointer_position: (0.0, 0.0),
        }
    }

    pub fn handle_event(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Keyboard(kb) => {
                match kb.state {
                    ButtonState::Pressed => {
                        self.pressed_keys.insert(kb.code);
                    }
                    ButtonState::Released => {
                        self.pressed_keys.remove(&kb.code);
                    }
                }
                self.modifiers = kb.modifiers;
            }
            InputEvent::Pointer(ptr) => {
                match &ptr.kind {
                    PointerEventKind::Moved => {
                        self.pointer_position = ptr.position;
                    }
                    PointerEventKind::ButtonPressed(button) => {
                        self.pressed_buttons.insert(*button);
                        self.pointer_position = ptr.position;
                    }
                    PointerEventKind::ButtonReleased(button) => {
                        self.pressed_buttons.remove(button);
                        self.pointer_position = ptr.position;
                    }
                    PointerEventKind::Scroll { .. } => {
                        self.pointer_position = ptr.position;
                    }
                    PointerEventKind::Entered | PointerEventKind::Left => {}
                }
                self.modifiers = ptr.modifiers;
            }
            InputEvent::FocusChanged(false) => {
                self.pressed_keys.clear();
                self.pressed_buttons.clear();
                self.modifiers = Modifiers::NONE;
            }
            InputEvent::FocusChanged(true) => {}
        }
    }

    pub fn is_key_pressed(&self, code: KeyCode) -> bool {
        self.pressed_keys.contains(&code)
    }

    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_buttons.contains(&button)
    }

    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn pointer_position(&self) -> (f64, f64) {
        self.pointer_position
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{KeyboardEvent, PointerEvent};
    use crate::key::Key;

    #[test]
    fn track_key_press_and_release() {
        let mut state = InputState::new();
        assert!(!state.is_key_pressed(KeyCode::KeyA));

        state.handle_event(&InputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("a".into()),
            code: KeyCode::KeyA,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            text: Some("a".into()),
            repeat: false,
        }));
        assert!(state.is_key_pressed(KeyCode::KeyA));

        state.handle_event(&InputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("a".into()),
            code: KeyCode::KeyA,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        }));
        assert!(!state.is_key_pressed(KeyCode::KeyA));
    }

    #[test]
    fn track_pointer_position() {
        let mut state = InputState::new();
        assert_eq!(state.pointer_position(), (0.0, 0.0));

        state.handle_event(&InputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Moved,
            position: (100.0, 200.0),
            modifiers: Modifiers::NONE,
        }));
        assert_eq!(state.pointer_position(), (100.0, 200.0));
    }

    #[test]
    fn track_mouse_button() {
        let mut state = InputState::new();
        assert!(!state.is_button_pressed(MouseButton::Left));

        state.handle_event(&InputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::ButtonPressed(MouseButton::Left),
            position: (50.0, 50.0),
            modifiers: Modifiers::NONE,
        }));
        assert!(state.is_button_pressed(MouseButton::Left));

        state.handle_event(&InputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::ButtonReleased(MouseButton::Left),
            position: (50.0, 50.0),
            modifiers: Modifiers::NONE,
        }));
        assert!(!state.is_button_pressed(MouseButton::Left));
    }

    #[test]
    fn focus_lost_clears_state() {
        let mut state = InputState::new();

        state.handle_event(&InputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("a".into()),
            code: KeyCode::KeyA,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            text: Some("a".into()),
            repeat: false,
        }));
        state.handle_event(&InputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::ButtonPressed(MouseButton::Left),
            position: (10.0, 20.0),
            modifiers: Modifiers::NONE,
        }));
        assert!(state.is_key_pressed(KeyCode::KeyA));
        assert!(state.is_button_pressed(MouseButton::Left));

        state.handle_event(&InputEvent::FocusChanged(false));
        assert!(!state.is_key_pressed(KeyCode::KeyA));
        assert!(!state.is_button_pressed(MouseButton::Left));
    }

    #[test]
    fn track_modifiers() {
        let mut state = InputState::new();

        let mods = Modifiers {
            shift: true,
            ctrl: false,
            alt: false,
            meta: false,
        };
        state.handle_event(&InputEvent::Keyboard(KeyboardEvent {
            key: Key::Named(crate::key::NamedKey::Shift),
            code: KeyCode::ShiftLeft,
            state: ButtonState::Pressed,
            modifiers: mods,
            text: None,
            repeat: false,
        }));
        assert!(state.modifiers().shift);
        assert!(!state.modifiers().ctrl);
    }
}
