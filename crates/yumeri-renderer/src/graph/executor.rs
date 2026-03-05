use ash::vk;

use super::compiler::CompiledGraph;
use super::pass::RenderPassContext;

pub(crate) struct GraphExecutor;

impl GraphExecutor {
    pub fn execute(
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        compiled: &mut CompiledGraph,
        swapchain_image: vk::Image,
        swapchain_image_view: vk::ImageView,
        extent: vk::Extent2D,
        _swapchain_format: vk::Format,
    ) {
        unsafe {
            // UNDEFINED -> COLOR_ATTACHMENT_OPTIMAL
            let image_barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                .src_access_mask(vk::AccessFlags2::NONE)
                .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image(swapchain_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dependency_info = vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&image_barrier));
            device.cmd_pipeline_barrier2(command_buffer, &dependency_info);

            for (pass_index, pass) in compiled.passes.iter_mut().enumerate() {
                let load_op = if pass_index == 0 {
                    vk::AttachmentLoadOp::CLEAR
                } else {
                    vk::AttachmentLoadOp::LOAD
                };

                let color_attachment = vk::RenderingAttachmentInfo::default()
                    .image_view(swapchain_image_view)
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(load_op)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 1.0],
                        },
                    });

                let rendering_info = vk::RenderingInfo::default()
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    })
                    .layer_count(1)
                    .color_attachments(std::slice::from_ref(&color_attachment));

                device.cmd_begin_rendering(command_buffer, &rendering_info);

                if let Some(execute_fn) = pass.execute_fn.take() {
                    let mut ctx = RenderPassContext {
                        device,
                        command_buffer,
                        render_area: extent,
                        color_attachment: swapchain_image_view,
                    };
                    execute_fn(&mut ctx);
                }

                device.cmd_end_rendering(command_buffer);
            }

            // COLOR_ATTACHMENT_OPTIMAL -> PRESENT_SRC_KHR
            let present_barrier = vk::ImageMemoryBarrier2::default()
                .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
                .dst_access_mask(vk::AccessFlags2::NONE)
                .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(swapchain_image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });

            let dependency_info = vk::DependencyInfo::default()
                .image_memory_barriers(std::slice::from_ref(&present_barrier));
            device.cmd_pipeline_barrier2(command_buffer, &dependency_info);
        }
    }
}
