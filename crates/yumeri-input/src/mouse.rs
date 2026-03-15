#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}

impl MouseButton {
    /// Convert from a Linux evdev button code (BTN_LEFT = 0x110, etc.).
    pub fn from_linux_evdev(code: u32) -> Self {
        match code {
            0x110 => Self::Left,
            0x111 => Self::Right,
            0x112 => Self::Middle,
            0x113 => Self::Back,
            0x114 => Self::Forward,
            other => Self::Other(other as u16),
        }
    }

    /// Convert to a Linux evdev button code.
    pub fn to_linux_evdev(self) -> u32 {
        match self {
            Self::Left => 0x110,
            Self::Right => 0x111,
            Self::Middle => 0x112,
            Self::Back => 0x113,
            Self::Forward => 0x114,
            Self::Other(id) => id as u32,
        }
    }
}
