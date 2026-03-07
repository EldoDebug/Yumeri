use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use yumeri_renderer::GpuContext;

pub struct CapturedFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Vulkan staging image for reading back rendered pixels.
///
/// This type owns GPU resources that cannot be freed automatically on drop
/// because cleanup requires access to `GpuContext`. Callers **must** call
/// [`FrameCapture::destroy`] before dropping this value to avoid leaking
/// the Vulkan image and its memory allocation.
pub struct FrameCapture {
    image: vk::Image,
    allocation: Option<Allocation>,
    width: u32,
    height: u32,
}

impl FrameCapture {
    pub fn new(gpu: &GpuContext, width: u32, height: u32) -> Result<Self, vk::Result> {
        let (image, allocation) = Self::create_staging_image(gpu, width, height)?;
        Ok(Self {
            image,
            allocation: Some(allocation),
            width,
            height,
        })
    }

    pub fn resize(&mut self, gpu: &GpuContext, width: u32, height: u32) -> Result<(), vk::Result> {
        self.destroy_image(gpu);
        let (image, allocation) = Self::create_staging_image(gpu, width, height)?;
        self.image = image;
        self.allocation = Some(allocation);
        self.width = width;
        self.height = height;
        Ok(())
    }

    pub fn record_copy(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        src_image: vk::Image,
        extent: vk::Extent2D,
    ) {
        let region = vk::ImageCopy {
            src_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_offset: vk::Offset3D::default(),
            dst_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            dst_offset: vk::Offset3D::default(),
            extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
        };

        unsafe {
            device.cmd_copy_image(
                cmd,
                src_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
    }

    pub fn read_pixels(&self) -> Result<CapturedFrame, vk::Result> {
        let allocation = self
            .allocation
            .as_ref()
            .expect("FrameCapture allocation missing");

        let size = (self.width * self.height * 4) as usize;
        let data = allocation
            .mapped_slice()
            .map(|slice| slice[..size].to_vec())
            .unwrap_or_else(|| vec![0u8; size]);

        Ok(CapturedFrame {
            data,
            width: self.width,
            height: self.height,
        })
    }

    fn create_staging_image(
        gpu: &GpuContext,
        width: u32,
        height: u32,
    ) -> Result<(vk::Image, Allocation), vk::Result> {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(vk::Format::R8G8B8A8_UNORM)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::LINEAR)
            .usage(vk::ImageUsageFlags::TRANSFER_DST)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let device = gpu.ash_device();
        let image = unsafe { device.create_image(&create_info, None)? };
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = gpu
            .allocator()
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name: "frame-capture-staging",
                requirements,
                location: MemoryLocation::GpuToCpu,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|_| vk::Result::ERROR_OUT_OF_DEVICE_MEMORY)?;

        unsafe {
            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        Ok((image, allocation))
    }

    fn destroy_image(&mut self, gpu: &GpuContext) {
        if let Some(alloc) = self.allocation.take() {
            unsafe {
                gpu.ash_device().destroy_image(self.image, None);
            }
            let _ = gpu
                .allocator()
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .free(alloc);
        }
    }

    pub fn destroy(mut self, gpu: &GpuContext) {
        self.destroy_image(gpu);
    }
}

impl Drop for FrameCapture {
    fn drop(&mut self) {
        if self.allocation.is_some() {
            eprintln!("FrameCapture dropped without calling destroy() — GPU resources leaked");
        }
    }
}
