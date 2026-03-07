use ash::vk;

use crate::error::Result;
use crate::gpu::GpuContext;

pub trait PostEffect: std::any::Any {
    fn name(&self) -> &str;
    fn pass_count(&self) -> u32;
    fn initialize(
        &mut self,
        gpu: &GpuContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<()>;
    fn record(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        pass_index: u32,
        descriptor_set: vk::DescriptorSet,
        extent: vk::Extent2D,
    );
    fn destroy(&mut self, device: &ash::Device);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}
