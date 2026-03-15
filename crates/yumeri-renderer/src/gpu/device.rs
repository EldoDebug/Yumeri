use std::ffi::CStr;

use ash::vk;

use crate::error::{RendererError, Result};

/// Vulkan Video decode extensions to enable when available.
const VIDEO_EXTENSIONS: &[&CStr] = &[
    vk::KHR_VIDEO_QUEUE_NAME,
    vk::KHR_VIDEO_DECODE_QUEUE_NAME,
    vk::KHR_VIDEO_DECODE_H264_NAME,
    vk::KHR_VIDEO_DECODE_H265_NAME,
    vk::KHR_VIDEO_DECODE_AV1_NAME,
];

#[derive(Clone, Copy)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
    /// Video decode queue family (None if Vulkan Video not supported).
    pub video_decode: Option<u32>,
}

pub struct VulkanDevice {
    device: ash::Device,
    physical_device: vk::PhysicalDevice,
    queue_family_indices: QueueFamilyIndices,
    graphics_queue: vk::Queue,
    #[allow(dead_code)] // Queue handle reserved for future Vulkan Video decode
    video_decode_queue: Option<vk::Queue>,
    enabled_video_extensions: Vec<&'static CStr>,
}

impl VulkanDevice {
    pub fn new(
        instance: &ash::Instance,
        surface_loader: &ash::khr::surface::Instance,
        surface: vk::SurfaceKHR,
    ) -> Result<Self> {
        let (physical_device, queue_family_indices) =
            pick_physical_device(instance, surface_loader, surface)?;

        // Check which video extensions are available
        let available_extensions =
            unsafe { instance.enumerate_device_extension_properties(physical_device)? };
        let available_names: Vec<&CStr> = available_extensions
            .iter()
            .map(|ext| unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) })
            .collect();

        let enabled_video_extensions: Vec<&'static CStr> = VIDEO_EXTENSIONS
            .iter()
            .filter(|&&ext| available_names.iter().any(|&name| name == ext))
            .copied()
            .collect();

        if !enabled_video_extensions.is_empty() {
            let names: Vec<_> = enabled_video_extensions
                .iter()
                .map(|e| e.to_string_lossy())
                .collect();
            log::info!("Vulkan Video extensions available: {}", names.join(", "));
        }

        // Build queue create infos
        let queue_priorities = [1.0f32];
        let mut queue_create_infos = vec![vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_indices.graphics)
            .queue_priorities(&queue_priorities)];

        if let Some(vd_idx) = queue_family_indices.video_decode {
            if vd_idx != queue_family_indices.graphics {
                queue_create_infos.push(
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(vd_idx)
                        .queue_priorities(&queue_priorities),
                );
            }
        }

        // Collect all device extensions
        let mut all_extensions: Vec<*const i8> = vec![ash::khr::swapchain::NAME.as_ptr()];
        for ext in &enabled_video_extensions {
            all_extensions.push(ext.as_ptr());
        }

        let mut vulkan_12_features = vk::PhysicalDeviceVulkan12Features::default()
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .runtime_descriptor_array(true)
            .shader_sampled_image_array_non_uniform_indexing(true)
            .timeline_semaphore(true);

        let mut vulkan_13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&all_extensions)
            .push_next(&mut vulkan_12_features)
            .push_next(&mut vulkan_13_features);

        let device =
            unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let graphics_queue =
            unsafe { device.get_device_queue(queue_family_indices.graphics, 0) };

        let video_decode_queue = queue_family_indices.video_decode.map(|idx| unsafe {
            device.get_device_queue(idx, 0)
        });

        let props = unsafe { instance.get_physical_device_properties(physical_device) };
        let name = unsafe { CStr::from_ptr(props.device_name.as_ptr()) };
        log::info!("Selected GPU: {}", name.to_string_lossy());

        Ok(Self {
            device,
            physical_device,
            queue_family_indices,
            graphics_queue,
            video_decode_queue,
            enabled_video_extensions,
        })
    }

    pub fn raw(&self) -> &ash::Device {
        &self.device
    }

    pub fn physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device
    }

    pub fn queue_family_indices(&self) -> QueueFamilyIndices {
        self.queue_family_indices
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub fn enabled_video_extensions(&self) -> &[&'static CStr] {
        &self.enabled_video_extensions
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            self.device.destroy_device(None);
        }
    }
}

fn pick_physical_device(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<(vk::PhysicalDevice, QueueFamilyIndices)> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    if devices.is_empty() {
        return Err(RendererError::NoSuitableGpu);
    }

    let mut best: Option<(vk::PhysicalDevice, QueueFamilyIndices, bool)> = None;

    for &pd in &devices {
        let Some(indices) = find_queue_families(instance, surface_loader, surface, pd) else {
            continue;
        };

        let props = unsafe { instance.get_physical_device_properties(pd) };
        let is_discrete = props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;

        match &best {
            Some((_, _, true)) if !is_discrete => {}
            _ => best = Some((pd, indices, is_discrete)),
        }

        if is_discrete {
            break;
        }
    }

    let (pd, indices, _) = best.ok_or(RendererError::NoSuitableGpu)?;
    Ok((pd, indices))
}

fn find_queue_families(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Option<QueueFamilyIndices> {
    let families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    let mut graphics = None;
    let mut video_decode = None;

    for (i, family) in families.iter().enumerate() {
        let i = i as u32;

        if graphics.is_none()
            && family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        {
            let present_support = unsafe {
                surface_loader
                    .get_physical_device_surface_support(physical_device, i, surface)
                    .unwrap_or(false)
            };
            if present_support {
                graphics = Some(i);
            }
        }

        // Detect video decode queue family
        if video_decode.is_none()
            && family
                .queue_flags
                .contains(vk::QueueFlags::VIDEO_DECODE_KHR)
        {
            video_decode = Some(i);
        }
    }

    graphics.map(|g| QueueFamilyIndices {
        graphics: g,
        video_decode,
    })
}
