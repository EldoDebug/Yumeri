#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Anchor {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
}

impl Anchor {
    pub const FILL: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: true,
    };
    pub const TOP: Self = Self {
        top: true,
        bottom: false,
        left: true,
        right: true,
    };
    pub const BOTTOM: Self = Self {
        top: false,
        bottom: true,
        left: true,
        right: true,
    };
    pub const LEFT: Self = Self {
        top: true,
        bottom: true,
        left: true,
        right: false,
    };
    pub const RIGHT: Self = Self {
        top: true,
        bottom: true,
        left: false,
        right: true,
    };
}
