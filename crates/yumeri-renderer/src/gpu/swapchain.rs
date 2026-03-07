use ash::vk;

use crate::error::Result;
use crate::gpu::surface::Surface;
use crate::gpu::GpuContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreferredPresentMode {
    /// VSync ON (FIFO)
    Fifo,
    /// Triple buffering (MAILBOX)
    #[default]
    Mailbox,
    /// VSync OFF (IMMEDIATE)
    Immediate,
}

impl PreferredPresentMode {
    fn to_vk(self) -> vk::PresentModeKHR {
        match self {
            Self::Fifo => vk::PresentModeKHR::FIFO,
            Self::Mailbox => vk::PresentModeKHR::MAILBOX,
            Self::Immediate => vk::PresentModeKHR::IMMEDIATE,
        }
    }
}

#[derive(Default)]
pub struct SwapchainConfig {
    pub preferred_present_mode: PreferredPresentMode,
    pub transparent: bool,
}

pub struct Swapchain {
    swapchain: vk::SwapchainKHR,
    swapchain_loader: ash::khr::swapchain::Device,
    device: ash::Device,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    present_mode: vk::PresentModeKHR,
}

impl Swapchain {
    pub fn new(
        gpu: &GpuContext,
        surface: &Surface,
        width: u32,
        height: u32,
        config: &SwapchainConfig,
    ) -> Result<Self> {
        let swapchain_loader =
            ash::khr::swapchain::Device::new(gpu.ash_instance(), gpu.ash_device());

        let (swapchain, format, extent, present_mode) =
            create_swapchain(gpu, surface, &swapchain_loader, width, height, vk::SwapchainKHR::null(), config)?;

        let images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let image_views = create_image_views(gpu.ash_device(), &images, format.format)?;

        Ok(Self {
            swapchain,
            swapchain_loader,
            device: gpu.ash_device().clone(),
            images,
            image_views,
            format,
            extent,
            present_mode,
        })
    }

    pub fn recreate(
        &mut self,
        gpu: &GpuContext,
        surface: &Surface,
        width: u32,
        height: u32,
        config: &SwapchainConfig,
    ) -> Result<()> {
        unsafe {
            let _ = gpu.ash_device().device_wait_idle();
        }

        destroy_image_views(gpu.ash_device(), &self.image_views);

        let old_swapchain = self.swapchain;
        let (swapchain, format, extent, present_mode) =
            create_swapchain(gpu, surface, &self.swapchain_loader, width, height, old_swapchain, config)?;

        unsafe {
            self.swapchain_loader
                .destroy_swapchain(old_swapchain, None);
        }

        let images = unsafe { self.swapchain_loader.get_swapchain_images(swapchain)? };
        let image_views = create_image_views(gpu.ash_device(), &images, format.format)?;

        self.swapchain = swapchain;
        self.images = images;
        self.image_views = image_views;
        self.format = format;
        self.extent = extent;
        self.present_mode = present_mode;

        Ok(())
    }

    pub fn acquire_next_image(&self, semaphore: vk::Semaphore) -> Result<(u32, bool)> {
        let (index, suboptimal) = unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                semaphore,
                vk::Fence::null(),
            )?
        };
        Ok((index, suboptimal))
    }

    pub fn raw(&self) -> vk::SwapchainKHR {
        self.swapchain
    }

    pub fn loader(&self) -> &ash::khr::swapchain::Device {
        &self.swapchain_loader
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }

    pub fn format(&self) -> vk::SurfaceFormatKHR {
        self.format
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }

    #[allow(dead_code)]
    pub fn present_mode(&self) -> vk::PresentModeKHR {
        self.present_mode
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for &view in &self.image_views {
                self.device.destroy_image_view(view, None);
            }
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

fn create_swapchain(
    gpu: &GpuContext,
    surface: &Surface,
    swapchain_loader: &ash::khr::swapchain::Device,
    width: u32,
    height: u32,
    old_swapchain: vk::SwapchainKHR,
    config: &SwapchainConfig,
) -> Result<(
    vk::SwapchainKHR,
    vk::SurfaceFormatKHR,
    vk::Extent2D,
    vk::PresentModeKHR,
)> {
    let physical_device = gpu.physical_device();
    let capabilities = surface.capabilities(physical_device)?;
    let formats = surface.formats(physical_device)?;
    let present_modes = surface.present_modes(physical_device)?;

    let format = formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .copied()
        .unwrap_or(formats[0]);

    let preferred_vk = config.preferred_present_mode.to_vk();
    let present_mode = if present_modes.contains(&preferred_vk) {
        preferred_vk
    } else {
        vk::PresentModeKHR::FIFO // guaranteed to be available
    };

    let extent = if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    };

    let mut image_count = capabilities.min_image_count + 1;
    if capabilities.max_image_count > 0 {
        image_count = image_count.min(capabilities.max_image_count);
    }

    let composite_alpha = if config.transparent {
        let supported = capabilities.supported_composite_alpha;
        if supported.contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        } else if supported.contains(vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED) {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        } else if supported.contains(vk::CompositeAlphaFlagsKHR::INHERIT) {
            vk::CompositeAlphaFlagsKHR::INHERIT
        } else {
            log::warn!("Transparent background requested but no transparent composite alpha mode is supported; falling back to OPAQUE");
            vk::CompositeAlphaFlagsKHR::OPAQUE
        }
    } else {
        vk::CompositeAlphaFlagsKHR::OPAQUE
    };

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface.raw())
        .min_image_count(image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | (capabilities.supported_usage_flags
                    & (vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST)),
        )
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .pre_transform(capabilities.current_transform)
        .composite_alpha(composite_alpha)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(old_swapchain);

    let swapchain = unsafe { swapchain_loader.create_swapchain(&create_info, None)? };

    Ok((swapchain, format, extent, present_mode))
}

fn create_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>> {
    images
        .iter()
        .map(|&image| {
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
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let view = unsafe { device.create_image_view(&info, None)? };
            Ok(view)
        })
        .collect()
}

fn destroy_image_views(device: &ash::Device, views: &[vk::ImageView]) {
    for &view in views {
        unsafe {
            device.destroy_image_view(view, None);
        }
    }
}
