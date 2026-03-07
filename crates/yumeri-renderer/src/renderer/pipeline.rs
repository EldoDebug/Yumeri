use ash::vk;

use crate::error::Result;

#[derive(Clone, Copy, Default)]
pub(crate) enum BlendMode {
    #[default]
    Alpha,
    Additive,
    Opaque,
    Multiply,
}

impl BlendMode {
    fn to_blend_attachment(self) -> vk::PipelineColorBlendAttachmentState {
        match self {
            BlendMode::Alpha => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            BlendMode::Additive => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            BlendMode::Opaque => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
            BlendMode::Multiply => vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::DST_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA),
        }
    }
}

pub(crate) struct GfxPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl GfxPipeline {
    pub fn owned_set_layout(&self, index: usize) -> vk::DescriptorSetLayout {
        self.descriptor_set_layouts
            .get(index)
            .copied()
            .unwrap_or_else(|| {
                panic!(
                    "owned_set_layout({index}) out of bounds (pipeline has {} owned layouts)",
                    self.descriptor_set_layouts.len()
                )
            })
    }

    pub fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.pipeline, None);
            device.destroy_pipeline_layout(self.pipeline_layout, None);
            for &layout in &self.descriptor_set_layouts {
                device.destroy_descriptor_set_layout(layout, None);
            }
        }
    }
}

pub(crate) struct PipelineBuilder<'a> {
    device: &'a ash::Device,
    color_format: vk::Format,
    vertex_spv: &'a [u8],
    fragment_spv: &'a [u8],
    topology: vk::PrimitiveTopology,
    blend_mode: BlendMode,
    custom_blend: Option<vk::PipelineColorBlendAttachmentState>,
    cull_mode: vk::CullModeFlags,
    push_constant_ranges: Vec<vk::PushConstantRange>,
    descriptor_bindings: Vec<Vec<vk::DescriptorSetLayoutBinding<'a>>>,
    external_set_layouts: Vec<vk::DescriptorSetLayout>,
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
}

impl<'a> PipelineBuilder<'a> {
    pub fn new(
        device: &'a ash::Device,
        color_format: vk::Format,
        vertex_spv: &'a [u8],
        fragment_spv: &'a [u8],
    ) -> Self {
        Self {
            device,
            color_format,
            vertex_spv,
            fragment_spv,
            topology: vk::PrimitiveTopology::TRIANGLE_STRIP,
            blend_mode: BlendMode::default(),
            custom_blend: None,
            cull_mode: vk::CullModeFlags::NONE,
            push_constant_ranges: Vec::new(),
            descriptor_bindings: Vec::new(),
            external_set_layouts: Vec::new(),
            vertex_bindings: Vec::new(),
            vertex_attributes: Vec::new(),
        }
    }

    pub fn topology(mut self, topology: vk::PrimitiveTopology) -> Self {
        self.topology = topology;
        self
    }

    pub fn blend_mode(mut self, mode: BlendMode) -> Self {
        self.blend_mode = mode;
        self
    }

    pub fn custom_blend(mut self, attachment: vk::PipelineColorBlendAttachmentState) -> Self {
        self.custom_blend = Some(attachment);
        self
    }

    pub fn cull_mode(mut self, mode: vk::CullModeFlags) -> Self {
        self.cull_mode = mode;
        self
    }

    pub fn vertex_input(
        mut self,
        bindings: Vec<vk::VertexInputBindingDescription>,
        attributes: Vec<vk::VertexInputAttributeDescription>,
    ) -> Self {
        self.vertex_bindings = bindings;
        self.vertex_attributes = attributes;
        self
    }

    pub fn push_constant(mut self, stage_flags: vk::ShaderStageFlags, offset: u32, size: u32) -> Self {
        self.push_constant_ranges.push(
            vk::PushConstantRange::default()
                .stage_flags(stage_flags)
                .offset(offset)
                .size(size),
        );
        self
    }

    pub fn descriptor_set(mut self, bindings: &[vk::DescriptorSetLayoutBinding<'a>]) -> Self {
        self.descriptor_bindings.push(bindings.to_vec());
        self
    }

    pub fn external_set_layout(mut self, layout: vk::DescriptorSetLayout) -> Self {
        self.external_set_layouts.push(layout);
        self
    }

    pub fn build(self) -> Result<GfxPipeline> {
        unsafe { self.build_inner() }
    }

    unsafe fn build_inner(self) -> Result<GfxPipeline> {
        let device = self.device;

        let vert_module = create_shader_module(device, self.vertex_spv)?;
        let frag_module = match create_shader_module(device, self.fragment_spv) {
            Ok(m) => m,
            Err(e) => {
                unsafe { device.destroy_shader_module(vert_module, None) };
                return Err(e);
            }
        };

        let result = unsafe {
            self.build_pipeline(device, vert_module, frag_module)
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        result
    }

    unsafe fn build_pipeline(
        &self,
        device: &ash::Device,
        vert_module: vk::ShaderModule,
        frag_module: vk::ShaderModule,
    ) -> Result<GfxPipeline> {
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

        // Create owned descriptor set layouts
        let mut owned_layouts = Vec::with_capacity(self.descriptor_bindings.len());
        for bindings in &self.descriptor_bindings {
            match unsafe {
                device.create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings),
                    None,
                )
            } {
                Ok(layout) => owned_layouts.push(layout),
                Err(e) => {
                    destroy_layouts(device, &owned_layouts);
                    return Err(e.into());
                }
            }
        }

        // Combine owned + external layouts for pipeline layout
        let mut all_set_layouts = owned_layouts.clone();
        all_set_layouts.extend_from_slice(&self.external_set_layouts);

        let pipeline_layout = match unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::default()
                    .set_layouts(&all_set_layouts)
                    .push_constant_ranges(&self.push_constant_ranges),
                None,
            )
        } {
            Ok(layout) => layout,
            Err(e) => {
                destroy_layouts(device, &owned_layouts);
                return Err(e.into());
            }
        };

        let vertex_input = if self.vertex_bindings.is_empty() {
            vk::PipelineVertexInputStateCreateInfo::default()
        } else {
            vk::PipelineVertexInputStateCreateInfo::default()
                .vertex_binding_descriptions(&self.vertex_bindings)
                .vertex_attribute_descriptions(&self.vertex_attributes)
        };

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(self.topology);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(self.cull_mode)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .line_width(1.0);

        let multisample = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let blend_attachment = self
            .custom_blend
            .unwrap_or_else(|| self.blend_mode.to_blend_attachment());
        let color_blend = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(std::slice::from_ref(&blend_attachment));

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let color_formats = [self.color_format];
        let mut rendering_info =
            vk::PipelineRenderingCreateInfo::default().color_attachment_formats(&color_formats);

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

        let pipeline = match unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_, e)| e)
        } {
            Ok(pipelines) => pipelines[0],
            Err(e) => {
                unsafe { device.destroy_pipeline_layout(pipeline_layout, None) };
                destroy_layouts(device, &owned_layouts);
                return Err(e.into());
            }
        };

        Ok(GfxPipeline {
            pipeline,
            pipeline_layout,
            descriptor_set_layouts: owned_layouts,
        })
    }
}

fn destroy_layouts(device: &ash::Device, layouts: &[vk::DescriptorSetLayout]) {
    for &layout in layouts {
        unsafe { device.destroy_descriptor_set_layout(layout, None) };
    }
}

pub(crate) fn create_shader_module(device: &ash::Device, spv: &[u8]) -> Result<vk::ShaderModule> {
    assert!(spv.len() % 4 == 0, "SPIR-V byte length must be a multiple of 4");
    let code: Vec<u32> = spv
        .chunks_exact(4)
        .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();
    let info = vk::ShaderModuleCreateInfo::default().code(&code);
    Ok(unsafe { device.create_shader_module(&info, None)? })
}
