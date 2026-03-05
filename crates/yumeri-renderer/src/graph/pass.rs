use ash::vk;

use super::resource::ResourceId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PassId(pub(crate) u32);

#[allow(dead_code)]
pub(crate) struct PassNode {
    pub id: PassId,
    pub name: String,
    pub reads: Vec<ResourceId>,
    pub writes: Vec<ResourceId>,
    pub execute_fn: Option<Box<dyn FnOnce(&mut RenderPassContext)>>,
}

#[allow(dead_code)]
pub struct RenderPassContext<'a> {
    pub(crate) device: &'a ash::Device,
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) render_area: vk::Extent2D,
    pub(crate) color_attachment: vk::ImageView,
}

impl RenderPassContext<'_> {
    pub fn device(&self) -> &ash::Device {
        self.device
    }

    pub fn command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }

    pub fn render_area(&self) -> vk::Extent2D {
        self.render_area
    }

    #[allow(dead_code)]
    pub fn color_attachment(&self) -> vk::ImageView {
        self.color_attachment
    }
}
