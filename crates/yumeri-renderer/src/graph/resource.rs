use ash::vk;

use super::pass::PassId;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ResourceId(pub(crate) u32);

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ImageDesc {
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
    pub usage: vk::ImageUsageFlags,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) enum ResourceDesc {
    Image(ImageDesc),
    Swapchain,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub(crate) struct VirtualResource {
    pub id: ResourceId,
    pub desc: ResourceDesc,
    pub written_by: Vec<PassId>,
    pub read_by: Vec<PassId>,
}
