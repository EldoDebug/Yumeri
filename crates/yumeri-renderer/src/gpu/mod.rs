mod instance;
mod device;
pub(crate) mod surface;
pub(crate) mod swapchain;

use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::error::Result;

pub use device::QueueFamilyIndices;

pub struct GpuContext {
    // Drop order: allocator before device, device before instance
    allocator: Arc<Mutex<Option<Allocator>>>,
    device: device::VulkanDevice,
    instance: instance::VulkanInstance,
}

impl GpuContext {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self> {
        let vk_instance = instance::VulkanInstance::new(display_handle)?;

        // Temporary surface needed for physical device selection (present support check)
        let temp_surface = unsafe {
            ash_window::create_surface(
                vk_instance.entry(),
                vk_instance.raw(),
                display_handle,
                window_handle,
                None,
            )?
        };
        let temp_surface_loader =
            ash::khr::surface::Instance::new(vk_instance.entry(), vk_instance.raw());

        let vk_device =
            device::VulkanDevice::new(vk_instance.raw(), &temp_surface_loader, temp_surface)?;

        unsafe {
            temp_surface_loader.destroy_surface(temp_surface, None);
        }

        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: vk_instance.raw().clone(),
            device: vk_device.raw().clone(),
            physical_device: vk_device.physical_device(),
            debug_settings: Default::default(),
            buffer_device_address: false,
            allocation_sizes: Default::default(),
        })
        .map_err(|e| crate::error::RendererError::AllocatorInit(e.to_string()))?;

        Ok(Self {
            allocator: Arc::new(Mutex::new(Some(allocator))),
            device: vk_device,
            instance: vk_instance,
        })
    }

    pub fn entry(&self) -> &ash::Entry {
        self.instance.entry()
    }

    pub fn ash_instance(&self) -> &ash::Instance {
        self.instance.raw()
    }

    pub fn ash_device(&self) -> &ash::Device {
        self.device.raw()
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.device.physical_device()
    }

    pub fn queue_family_indices(&self) -> QueueFamilyIndices {
        self.device.queue_family_indices()
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.device.graphics_queue()
    }

    pub fn allocator(&self) -> &Arc<Mutex<Option<Allocator>>> {
        &self.allocator
    }

    pub fn submit_oneshot(
        &self,
        record: impl FnOnce(vk::CommandBuffer),
    ) -> crate::error::Result<()> {
        let device = self.ash_device();
        unsafe {
            let pool_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(self.queue_family_indices().graphics);
            let pool = device.create_command_pool(&pool_info, None)?;

            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let cmd = device.allocate_command_buffers(&alloc_info)?[0];

            let begin_info = vk::CommandBufferBeginInfo::default()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            device.begin_command_buffer(cmd, &begin_info)?;

            record(cmd);

            device.end_command_buffer(cmd)?;

            let fence = device
                .create_fence(&vk::FenceCreateInfo::default(), None)?;

            let submit_info =
                vk::SubmitInfo::default().command_buffers(std::slice::from_ref(&cmd));
            device.queue_submit(self.graphics_queue(), &[submit_info], fence)?;
            device.wait_for_fences(&[fence], true, u64::MAX)?;

            device.destroy_fence(fence, None);
            device.destroy_command_pool(pool, None);
        }
        Ok(())
    }
}

impl Drop for GpuContext {
    fn drop(&mut self) {
        unsafe {
            let _ = self.ash_device().device_wait_idle();
        }
        // Drop allocator before device
        if let Ok(mut guard) = self.allocator.lock() {
            drop(guard.take());
        }
    }
}
