use bitflags::bitflags;
use slotmap::new_key_type;

use crate::renderer::renderer2d::shapes::{
    pack_instance, Color, ShapeType, FLOATS_PER_INSTANCE,
};
use crate::texture::{Texture, TextureId};

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
    pub(crate) texture: Option<Texture>,
    pub(crate) visible: bool,
    pub(crate) z_index: i32,

    pub(crate) translate: [f32; 2],
    pub(crate) scale: [f32; 2],
    pub(crate) rotation: f32,

    pub(crate) world_position: [f32; 2],
    pub(crate) render_index: Option<u32>,

    pub(crate) dirty: DirtyFlags,

    pub(crate) text_glyph_children: Vec<NodeId>,
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
            texture: None,
            visible: true,
            z_index: 0,
            translate: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation: 0.0,
            world_position: [0.0, 0.0],
            render_index: None,
            dirty: DirtyFlags::all(),
            text_glyph_children: Vec::new(),
        }
    }

    pub(crate) fn is_renderable(&self) -> bool {
        self.visible && self.shape_type != ShapeType::None
    }

    pub(crate) fn to_instance_data(
        &self,
        resolve: impl Fn(TextureId) -> u32,
    ) -> [f32; FLOATS_PER_INSTANCE] {
        let (cos_r, sin_r) = (self.rotation.cos(), self.rotation.sin());
        pack_instance(
            self.world_position,
            self.size,
            self.corner_radius,
            self.shape_type,
            self.color,
            self.texture,
            cos_r,
            sin_r,
            self.scale,
            resolve,
        )
    }
}
