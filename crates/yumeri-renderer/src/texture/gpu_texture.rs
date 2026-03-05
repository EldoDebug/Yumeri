use ash::vk;

use crate::resource::Image;

pub(crate) struct GpuTexture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor_index: u32,
}
