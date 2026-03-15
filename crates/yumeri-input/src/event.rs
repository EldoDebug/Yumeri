use crate::key::Key;
use crate::key::KeyCode;
use crate::mouse::MouseButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    pub fn is_pressed(self) -> bool {
        self == Self::Pressed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    };
}

#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    pub key: Key,
    pub code: KeyCode,
    pub state: ButtonState,
    pub modifiers: Modifiers,
    pub text: Option<String>,
    pub repeat: bool,
}

#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub kind: PointerEventKind,
    pub position: (f64, f64),
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone)]
pub enum PointerEventKind {
    Moved,
    ButtonPressed(MouseButton),
    ButtonReleased(MouseButton),
    Scroll { delta_x: f64, delta_y: f64 },
    Entered,
    Left,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Keyboard(KeyboardEvent),
    Pointer(PointerEvent),
    FocusChanged(bool),
}
