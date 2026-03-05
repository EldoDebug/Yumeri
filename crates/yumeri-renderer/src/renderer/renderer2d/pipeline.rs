use ash::vk;

use crate::error::Result;

const VERT_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sdf_2d.vert.spv"));
const FRAG_SPV: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/sdf_2d.frag.spv"));

pub(crate) struct Pipeline2D {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub ssbo_descriptor_set_layout: vk::DescriptorSetLayout,
}

impl Pipeline2D {
    pub fn new(
        device: &ash::Device,
        color_format: vk::Format,
        texture_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        unsafe {
            let vert_module = Self::create_shader_module(device, VERT_SPV)?;
            let frag_module = Self::create_shader_module(device, FRAG_SPV)?;

            let entry_point = c"main";

            let shader_stages = [
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vert_module)
                    .name(entry_point),
                vk::PipelineShaderStageCreateInfo::default()
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(frag_module)
                    .name(entry_point),
            ];

            // Set 0: SSBO at binding 0
            let ssbo_binding = vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX);

            let ssbo_descriptor_set_layout = device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&ssbo_binding)),
                None,
            )?;

            // Push constants: viewport_size (2 x f32 = 8 bytes)
            let push_constant_range = vk::PushConstantRange::default()
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .offset(0)
                .size(8);

            // Set 0 = SSBO, Set 1 = texture array
            let set_layouts = [ssbo_descriptor_set_layout, texture_descriptor_set_layout];
            let pipeline_layout = device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&set_layouts)
                    .push_constant_ranges(std::slice::from_ref(&push_constant_range)),
                None,
            )?;

            // Vertex input: none (attributeless rendering)
            let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

            // Input assembly: triangle strip
            let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_STRIP);

            // Viewport/scissor: dynamic
            let viewport_state = vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1);

            // Rasterization
            let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0);

            // Multisample: disabled
            let multisample = vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);

            // Color blend: standard alpha blending
            let blend_attachment = vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA);

            let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
                .attachments(std::slice::from_ref(&blend_attachment));

            // Dynamic state
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state =
                vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

            // Dynamic rendering (Vulkan 1.3)
            let color_formats = [color_format];
            let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
                .color_attachment_formats(&color_formats);

            let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
                .stages(&shader_stages)
                .vertex_input_state(&vertex_input)
                .input_assembly_state(&input_assembly)
                .viewport_state(&viewport_state)
                .rasterization_state(&rasterization)
                .multisample_state(&multisample)
                .color_blend_state(&color_blend)
                .dynamic_state(&dynamic_state)
                .layout(pipeline_layout)
                .push_next(&mut rendering_info);

            let pipeline = device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| e)?[0];

            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);

            Ok(Self {
                pipeline,
                pipeline_layout,
                ssbo_descriptor_set_layout,
            })
        }
    }

    fn create_shader_module(device: &ash::Device, spv: &[u8]) -> Result<vk::ShaderModule> {
        // Copy into an aligned Vec<u32> to avoid UB from include_bytes! alignment
        assert!(spv.len() % 4 == 0, "SPIR-V byte length must be a multiple of 4");
        let code: Vec<u32> = spv
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        let info = vk::ShaderModuleCreateInfo::default().code(&code);
        Ok(unsafe { device.create_shader_module(&info, None)? })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            device.destroy_descriptor_set_layout(self.ssbo_descriptor_set_layout, None);
        }
    }
}
