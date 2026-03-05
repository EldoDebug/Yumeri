use std::sync::{Arc, Mutex};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, Allocator, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;

use crate::error::Result;
use crate::gpu::GpuContext;

pub struct Buffer {
    buffer: vk::Buffer,
    allocation: Option<Allocation>,
    device: ash::Device,
    allocator: Arc<Mutex<Option<Allocator>>>,
}

impl Buffer {
    pub fn new(
        gpu: &GpuContext,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
    ) -> Result<Self> {
        let buffer_info = vk::BufferCreateInfo::default().size(size).usage(usage);

        let device = gpu.ash_device();
        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = gpu
            .allocator()
            .lock()
            .unwrap()
            .as_mut()
            .expect("allocator dropped")
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })?;

        unsafe {
            device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())?;
        }

        Ok(Self {
            buffer,
            allocation: Some(allocation),
            device: device.clone(),
            allocator: Arc::clone(gpu.allocator()),
        })
    }

    pub fn raw(&self) -> vk::Buffer {
        self.buffer
    }

    pub fn mapped_slice_mut(&mut self) -> Option<&mut [u8]> {
        self.allocation
            .as_mut()
            .and_then(|a| a.mapped_slice_mut())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        if let Some(allocation) = self.allocation.take() {
            if let Ok(mut guard) = self.allocator.lock() {
                if let Some(alloc) = guard.as_mut() {
                    let _ = alloc.free(allocation);
                }
            }
        }
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
    }
}
