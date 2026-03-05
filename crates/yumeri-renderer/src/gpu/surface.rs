use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::error::Result;
use crate::gpu::GpuContext;

pub struct Surface {
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
}

impl Surface {
    pub fn new(
        gpu: &GpuContext,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
    ) -> Result<Self> {
        let surface = unsafe {
            ash_window::create_surface(
                gpu.entry(),
                gpu.ash_instance(),
                display_handle,
                window_handle,
                None,
            )?
        };

        let surface_loader = ash::khr::surface::Instance::new(gpu.entry(), gpu.ash_instance());

        Ok(Self {
            surface,
            surface_loader,
        })
    }

    pub fn raw(&self) -> vk::SurfaceKHR {
        self.surface
    }

    pub fn capabilities(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<vk::SurfaceCapabilitiesKHR> {
        Ok(unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)?
        })
    }

    pub fn formats(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::SurfaceFormatKHR>> {
        Ok(unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)?
        })
    }

    pub fn present_modes(
        &self,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Vec<vk::PresentModeKHR>> {
        Ok(unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)?
        })
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
