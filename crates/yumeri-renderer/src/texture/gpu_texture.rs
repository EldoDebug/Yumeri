use ash::vk;

use crate::resource::Image;

pub(crate) struct GpuTexture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor_index: u32,
    /// When set, used instead of `image.view()` in descriptor updates.
    /// Used for GPU-decoded video frames where the RGBA8 output image
    /// is owned by YuvConverter's ring buffer.
    pub override_view: Option<vk::ImageView>,
}
