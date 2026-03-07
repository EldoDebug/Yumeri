use ash::vk;
use bytemuck::{Pod, Zeroable};

use crate::error::{RendererError, Result};
use crate::gpu::GpuContext;
use crate::renderer::pipeline::create_shader_module;

use super::effect::PostEffect;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GrayscalePushConstants {
    intensity: f32,
    mask_enabled: u32,
}

pub struct Grayscale {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    intensity: f32,
    mask_enabled: bool,
}

impl Grayscale {
    pub const NAME: &str = "grayscale";

    pub fn new() -> Self {
        Self {
            pipeline: vk::Pipeline::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            intensity: 1.0,
            mask_enabled: false,
        }
    }

    pub fn intensity(&self) -> f32 {
        self.intensity
    }

    pub fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity.clamp(0.0, 1.0);
    }

    pub fn set_mask_enabled(&mut self, enabled: bool) {
        self.mask_enabled = enabled;
    }
}

impl PostEffect for Grayscale {
    fn name(&self) -> &str {
        Self::NAME
    }

    fn pass_count(&self) -> u32 {
        1
    }

    fn initialize(
        &mut self,
        gpu: &GpuContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<()> {
        let device = gpu.ash_device();

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(std::mem::size_of::<GrayscalePushConstants>() as u32);

        let layouts = [descriptor_set_layout];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&layouts)
            .push_constant_ranges(std::slice::from_ref(&push_constant_range));
        self.pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let shader_module = create_shader_module(
            device,
            include_bytes!(concat!(env!("OUT_DIR"), "/postfx_grayscale.comp.spv")),
        )?;

        let entry_point = c"main";
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entry_point);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(self.pipeline_layout);

        self.pipeline = unsafe {
            device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_pipelines, err)| RendererError::Vulkan(err))?[0]
        };

        unsafe {
            device.destroy_shader_module(shader_module, None);
        }

        Ok(())
    }

    fn record(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        _pass_index: u32,
        descriptor_set: vk::DescriptorSet,
        extent: vk::Extent2D,
    ) {
        let push = GrayscalePushConstants {
            intensity: self.intensity,
            mask_enabled: u32::from(self.mask_enabled),
        };

        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, self.pipeline);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &[descriptor_set],
                &[],
            );
            device.cmd_push_constants(
                cmd,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                bytemuck::bytes_of(&push),
            );

            let group_x = (extent.width + 15) / 16;
            let group_y = (extent.height + 15) / 16;
            device.cmd_dispatch(cmd, group_x, group_y, 1);
        }
    }

    fn destroy(&mut self, device: &ash::Device) {
        unsafe {
            if self.pipeline != vk::Pipeline::null() {
                device.destroy_pipeline(self.pipeline, None);
                self.pipeline = vk::Pipeline::null();
            }
            if self.pipeline_layout != vk::PipelineLayout::null() {
                device.destroy_pipeline_layout(self.pipeline_layout, None);
                self.pipeline_layout = vk::PipelineLayout::null();
            }
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
