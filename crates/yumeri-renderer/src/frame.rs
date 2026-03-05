use ash::vk;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::gpu::swapchain::Swapchain;

pub(crate) const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct FrameContext {
    pub command_buffer: vk::CommandBuffer,
    pub image_available_semaphore: vk::Semaphore,
    pub render_finished_semaphore: vk::Semaphore,
    pub swapchain_image_index: u32,
}

struct PerFrame {
    fence: vk::Fence,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
}

pub struct FrameSynchronizer {
    frames: Vec<PerFrame>,
    current_frame: usize,
    device: ash::Device,
}

impl FrameSynchronizer {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
        let device = gpu.ash_device();
        let mut frames = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let fence_info =
                vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);
            let fence = unsafe { device.create_fence(&fence_info, None)? };

            let sem_info = vk::SemaphoreCreateInfo::default();
            let image_available_semaphore =
                unsafe { device.create_semaphore(&sem_info, None)? };
            let render_finished_semaphore =
                unsafe { device.create_semaphore(&sem_info, None)? };

            let pool_info = vk::CommandPoolCreateInfo::default()
                .flags(vk::CommandPoolCreateFlags::TRANSIENT)
                .queue_family_index(gpu.queue_family_indices().graphics);
            let command_pool = unsafe { device.create_command_pool(&pool_info, None)? };

            let alloc_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);
            let command_buffer =
                unsafe { device.allocate_command_buffers(&alloc_info)? }[0];

            frames.push(PerFrame {
                fence,
                image_available_semaphore,
                render_finished_semaphore,
                command_pool,
                command_buffer,
            });
        }

        Ok(Self {
            frames,
            current_frame: 0,
            device: device.clone(),
        })
    }

    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    pub fn begin_frame(
        &mut self,
        gpu: &GpuContext,
        swapchain: &Swapchain,
    ) -> Result<FrameContext> {
        let frame = &self.frames[self.current_frame];
        let device = gpu.ash_device();

        unsafe {
            device.wait_for_fences(&[frame.fence], true, u64::MAX)?;
        }

        let (image_index, _suboptimal) =
            swapchain.acquire_next_image(frame.image_available_semaphore)?;

        unsafe {
            device.reset_fences(&[frame.fence])?;
            device.reset_command_pool(frame.command_pool, vk::CommandPoolResetFlags::empty())?;
        }

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            device.begin_command_buffer(frame.command_buffer, &begin_info)?;
        }

        Ok(FrameContext {
            command_buffer: frame.command_buffer,
            image_available_semaphore: frame.image_available_semaphore,
            render_finished_semaphore: frame.render_finished_semaphore,
            swapchain_image_index: image_index,
        })
    }

    pub fn end_frame(
        &mut self,
        gpu: &GpuContext,
        swapchain: &Swapchain,
        frame_ctx: &FrameContext,
    ) -> Result<bool> {
        let frame = &self.frames[self.current_frame];
        let device = gpu.ash_device();

        unsafe {
            device.end_command_buffer(frame_ctx.command_buffer)?;
        }

        let wait_semaphores = [frame_ctx.image_available_semaphore];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [frame_ctx.render_finished_semaphore];
        let command_buffers = [frame_ctx.command_buffer];

        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            device.queue_submit(gpu.graphics_queue(), &[submit_info], frame.fence)?;
        }

        let swapchains = [swapchain.raw()];
        let image_indices = [frame_ctx.swapchain_image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let needs_recreate = unsafe {
            swapchain
                .loader()
                .queue_present(gpu.graphics_queue(), &present_info)
        };

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        match needs_recreate {
            Ok(suboptimal) => Ok(suboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for FrameSynchronizer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();
            for frame in &self.frames {
                self.device.destroy_fence(frame.fence, None);
                self.device
                    .destroy_semaphore(frame.image_available_semaphore, None);
                self.device
                    .destroy_semaphore(frame.render_finished_semaphore, None);
                self.device.destroy_command_pool(frame.command_pool, None);
            }
        }
    }
}
