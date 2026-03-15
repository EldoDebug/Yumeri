use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, Allocator, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;

use crate::error::Result;
use crate::gpu::GpuContext;

pub struct Image {
    image: vk::Image,
    view: vk::ImageView,
    allocation: Option<Allocation>,
    device: ash::Device,
    allocator: Arc<Mutex<Option<Allocator>>>,
}

impl Image {
    pub fn new(
        gpu: &GpuContext,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        location: MemoryLocation,
    ) -> Result<Self> {
        Self::new_with_flags(gpu, width, height, format, usage, location, vk::ImageCreateFlags::empty())
    }

    pub fn new_with_flags(
        gpu: &GpuContext,
        width: u32,
        height: u32,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
        location: MemoryLocation,
        flags: vk::ImageCreateFlags,
    ) -> Result<Self> {
        let image_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .flags(flags)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let device = gpu.ash_device();
        let image = unsafe { device.create_image(&image_info, None)? };
        let requirements = unsafe { device.get_image_memory_requirements(image) };

        let allocation = gpu
            .allocator()
            .lock()
            .unwrap()
            .as_mut()
            .expect("allocator dropped")
            .allocate(&AllocationCreateDesc {
                name: "image",
                requirements,
                location,
                linear: false,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            device.bind_image_memory(image, allocation.memory(), allocation.offset())?;
        }

        let aspect = if format == vk::Format::D32_SFLOAT
            || format == vk::Format::D24_UNORM_S8_UINT
            || format == vk::Format::D16_UNORM
        {
            vk::ImageAspectFlags::DEPTH
        } else {
            vk::ImageAspectFlags::COLOR
        };

        let view = create_image_view(device, image, format, aspect)?;

        Ok(Self {
            image,
            view,
            allocation: Some(allocation),
            device: device.clone(),
            allocator: Arc::clone(gpu.allocator()),
        })
    }

    pub fn raw(&self) -> vk::Image {
        self.image
    }

    pub fn view(&self) -> vk::ImageView {
        self.view
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.view, None);
        }

        if let Some(allocation) = self.allocation.take() {
            unsafe {
                self.device.destroy_image(self.image, None);
            }
            if let Ok(mut guard) = self.allocator.lock() {
                if let Some(alloc) = guard.as_mut() {
                    let _ = alloc.free(allocation);
                }
            }
        }
        // If allocation is None, this is a swapchain image -- don't destroy the vk::Image
    }
}

pub(crate) fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
    aspect: vk::ImageAspectFlags,
) -> Result<vk::ImageView> {
    let info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(vk::ComponentMapping {
            r: vk::ComponentSwizzle::IDENTITY,
            g: vk::ComponentSwizzle::IDENTITY,
            b: vk::ComponentSwizzle::IDENTITY,
            a: vk::ComponentSwizzle::IDENTITY,
        })
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: aspect,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let view = unsafe { device.create_image_view(&info, None)? };
    Ok(view)
}
