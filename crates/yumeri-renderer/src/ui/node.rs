use bitflags::bitflags;
use slotmap::new_key_type;

use crate::renderer::renderer2d::shapes::{
    pack_instance, Color, ShapeType, FLOATS_PER_INSTANCE,
};

new_key_type! { pub struct NodeId; }

bitflags! {
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct DirtyFlags: u8 {
        const TRANSFORM = 0b0001;
        const VISUAL    = 0b0010;
        const TREE      = 0b0100;
    }
}

pub(crate) struct Node {
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: Vec<NodeId>,

    pub(crate) position: [f32; 2],
    pub(crate) size: [f32; 2],
    pub(crate) corner_radius: f32,
    pub(crate) shape_type: ShapeType,
    pub(crate) color: Color,
    pub(crate) visible: bool,
    pub(crate) z_index: i32,

    pub(crate) world_position: [f32; 2],
    pub(crate) render_index: Option<u32>,

    pub(crate) dirty: DirtyFlags,
}

impl Node {
    pub(crate) fn new(shape_type: ShapeType) -> Self {
        Self {
            parent: None,
            children: Vec::new(),
            position: [0.0, 0.0],
            size: [0.0, 0.0],
            corner_radius: 0.0,
            shape_type,
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            visible: true,
            z_index: 0,
            world_position: [0.0, 0.0],
            render_index: None,
            dirty: DirtyFlags::all(),
        }
    }

    pub(crate) fn is_renderable(&self) -> bool {
        self.visible && self.shape_type != ShapeType::None
    }

    pub(crate) fn to_instance_data(&self) -> [f32; FLOATS_PER_INSTANCE] {
        pack_instance(
            self.world_position,
            self.size,
            self.corner_radius,
            self.shape_type,
            self.color,
        )
    }
}
