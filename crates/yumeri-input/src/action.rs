use crate::event::{ButtonState, InputEvent, Modifiers, PointerEventKind};
use crate::key::Key;
use crate::key::KeyCode;
use crate::mouse::MouseButton;

#[derive(Debug, Clone)]
pub struct InputMap<A> {
    bindings: Vec<Binding<A>>,
}

#[derive(Debug, Clone)]
pub struct Binding<A> {
    pub trigger: InputTrigger,
    pub action: A,
}

#[derive(Debug, Clone)]
pub enum InputTrigger {
    Key {
        key: Key,
        modifiers: Modifiers,
    },
    KeyCode {
        code: KeyCode,
        modifiers: Modifiers,
    },
    MouseButton {
        button: MouseButton,
        modifiers: Modifiers,
    },
}

impl<A> InputMap<A> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }

    pub fn bind(mut self, trigger: InputTrigger, action: A) -> Self {
        self.bindings.push(Binding { trigger, action });
        self
    }

    pub fn lookup(&self, event: &InputEvent) -> Option<&A> {
        for binding in &self.bindings {
            if binding.trigger.matches(event) {
                return Some(&binding.action);
            }
        }
        None
    }
}

impl<A> Default for InputMap<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl InputTrigger {
    fn matches(&self, event: &InputEvent) -> bool {
        match (self, event) {
            (
                InputTrigger::Key { key, modifiers },
                InputEvent::Keyboard(kb),
            ) => {
                kb.state == ButtonState::Pressed
                    && &kb.key == key
                    && *modifiers == kb.modifiers
            }
            (
                InputTrigger::KeyCode { code, modifiers },
                InputEvent::Keyboard(kb),
            ) => {
                kb.state == ButtonState::Pressed
                    && &kb.code == code
                    && *modifiers == kb.modifiers
            }
            (
                InputTrigger::MouseButton { button, modifiers },
                InputEvent::Pointer(ptr),
            ) => {
                matches!(&ptr.kind, PointerEventKind::ButtonPressed(b) if b == button)
                    && *modifiers == ptr.modifiers
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{KeyboardEvent, PointerEvent};
    use crate::key::NamedKey;

    #[derive(Debug, Clone, PartialEq)]
    enum Action {
        Jump,
        Shoot,
        Quit,
    }

    #[test]
    fn lookup_key_binding() {
        let map = InputMap::new()
            .bind(
                InputTrigger::Key {
                    key: Key::Named(NamedKey::Space),
                    modifiers: Modifiers::NONE,
                },
                Action::Jump,
            )
            .bind(
                InputTrigger::Key {
                    key: Key::Named(NamedKey::Escape),
                    modifiers: Modifiers::NONE,
                },
                Action::Quit,
            );

        let event = InputEvent::Keyboard(KeyboardEvent {
            key: Key::Named(NamedKey::Space),
            code: KeyCode::Space,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        });
        assert_eq!(map.lookup(&event), Some(&Action::Jump));

        let event = InputEvent::Keyboard(KeyboardEvent {
            key: Key::Named(NamedKey::Escape),
            code: KeyCode::Escape,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        });
        assert_eq!(map.lookup(&event), Some(&Action::Quit));
    }

    #[test]
    fn lookup_with_modifiers() {
        let mods = Modifiers {
            shift: false,
            ctrl: true,
            alt: false,
            meta: false,
        };
        let map = InputMap::new().bind(
            InputTrigger::Key {
                key: Key::Character("s".into()),
                modifiers: mods,
            },
            Action::Shoot,
        );

        // Without ctrl: no match
        let event = InputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("s".into()),
            code: KeyCode::KeyS,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        });
        assert_eq!(map.lookup(&event), None);

        // With ctrl: match
        let event = InputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("s".into()),
            code: KeyCode::KeyS,
            state: ButtonState::Pressed,
            modifiers: mods,
            text: None,
            repeat: false,
        });
        assert_eq!(map.lookup(&event), Some(&Action::Shoot));
    }

    #[test]
    fn lookup_mouse_button() {
        let map = InputMap::new().bind(
            InputTrigger::MouseButton {
                button: MouseButton::Left,
                modifiers: Modifiers::NONE,
            },
            Action::Shoot,
        );

        let event = InputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::ButtonPressed(MouseButton::Left),
            position: (0.0, 0.0),
            modifiers: Modifiers::NONE,
        });
        assert_eq!(map.lookup(&event), Some(&Action::Shoot));
    }

    #[test]
    fn no_match_on_release() {
        let map = InputMap::new().bind(
            InputTrigger::Key {
                key: Key::Named(NamedKey::Space),
                modifiers: Modifiers::NONE,
            },
            Action::Jump,
        );

        let event = InputEvent::Keyboard(KeyboardEvent {
            key: Key::Named(NamedKey::Space),
            code: KeyCode::Space,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
            text: None,
            repeat: false,
        });
        assert_eq!(map.lookup(&event), None);
    }
}
