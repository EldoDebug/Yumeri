pub mod focus;
pub mod hit_test;
pub mod propagation;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    Click,
    MouseDown,
    MouseUp,
    MouseEnter,
    MouseLeave,
    Focus,
    Blur,
    KeyDown,
    TextInput,
    Scroll,
}

#[derive(Clone, Debug)]
pub enum EventPayload {
    Click,
    MouseDown {
        x: f32,
        y: f32,
        button: yumeri_input::MouseButton,
    },
    MouseUp {
        x: f32,
        y: f32,
        button: yumeri_input::MouseButton,
    },
    MouseEnter,
    MouseLeave,
    Focus,
    Blur,
    KeyDown {
        event: yumeri_input::KeyboardEvent,
    },
    TextInput {
        text: String,
    },
    Scroll {
        delta_x: f32,
        delta_y: f32,
    },
}

impl EventPayload {
    pub fn kind(&self) -> EventKind {
        match self {
            Self::Click => EventKind::Click,
            Self::MouseDown { .. } => EventKind::MouseDown,
            Self::MouseUp { .. } => EventKind::MouseUp,
            Self::MouseEnter => EventKind::MouseEnter,
            Self::MouseLeave => EventKind::MouseLeave,
            Self::Focus => EventKind::Focus,
            Self::Blur => EventKind::Blur,
            Self::KeyDown { .. } => EventKind::KeyDown,
            Self::TextInput { .. } => EventKind::TextInput,
            Self::Scroll { .. } => EventKind::Scroll,
        }
    }
}
