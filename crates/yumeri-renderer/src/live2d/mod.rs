mod clipping;
pub mod coords;

pub use clipping::{ClippingManager, RectF, channel_flag_as_color};

use ash::vk;
use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use gpu_allocator::MemoryLocation;
use yumeri_live2d::core;

use crate::error::{RendererError, Result};
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};
use crate::renderer::pipeline::{GfxPipeline, PipelineBuilder};
use crate::resource::{Buffer, Image};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
    clip: [[f32; 4]; 4],
    base_color: [f32; 4],
    multiply_color: [f32; 4],
    screen_color: [f32; 4],
    channel_flag: [f32; 4],
    flags: [f32; 4],
}

const UNIFORM_SIZE: u64 = std::mem::size_of::<Uniforms>() as u64;
// Vulkan spec guarantees minUniformBufferOffsetAlignment <= 256
const UNIFORM_STRIDE: u64 = 256;

#[derive(Debug, Clone, Copy)]
pub struct RendererOptions {
    pub use_high_precision_mask: bool,
    pub premultiplied_alpha: bool,
    pub reset_dynamic_flags: bool,
}

impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            use_high_precision_mask: false,
            premultiplied_alpha: false,
            reset_dynamic_flags: true,
        }
    }
}

struct MeshBuffers {
    vertex: Buffer,
    index: Buffer,
    vertex_capacity: usize,
    index_capacity: usize,
    index_count: usize,
    vertex_uploaded: bool,
    index_uploaded: bool,
}

const MASK_ATLAS_SIZE: u32 = 256;

struct Pipelines {
    mask_setup_cull: GfxPipeline,
    mask_setup_no_cull: GfxPipeline,

    // 6 draw pipelines: 3 blend modes × {cull, no_cull}
    // Masked vs unmasked is handled by shader uniform (flags.y), not pipeline state.
    normal_cull: GfxPipeline,
    normal_no_cull: GfxPipeline,
    add_cull: GfxPipeline,
    add_no_cull: GfxPipeline,
    mult_cull: GfxPipeline,
    mult_no_cull: GfxPipeline,
}

#[derive(Clone, Copy)]
enum Live2DBlendMode {
    Normal,
    Add,
    Multiply,
}

pub struct Live2DRenderer {
    mask_image: Image,
    sampler: vk::Sampler,
    mask_descriptor_set: vk::DescriptorSet,

    // Shared layout for both texture and mask descriptor sets (identical bindings)
    sampler_descriptor_set_layout: vk::DescriptorSetLayout,
    texture_descriptor_sets: Vec<vk::DescriptorSet>,
    // Kept alive so Drop doesn't destroy Vulkan images while descriptor sets reference them
    #[allow(dead_code)]
    texture_images: Vec<Image>,

    uniform_buffer: Buffer,
    uniform_capacity: u64,
    uniform_descriptor_set_layout: vk::DescriptorSetLayout,
    uniform_descriptor_set: vk::DescriptorSet,

    descriptor_pool: vk::DescriptorPool,
    pipelines: Pipelines,
    meshes: Vec<MeshBuffers>,
    sorted_drawables: Vec<usize>,

    pub clip_manager: ClippingManager,
    options: RendererOptions,
    device: ash::Device,
}

impl Live2DRenderer {
    pub fn from_model(
        gpu: &GpuContext,
        swapchain_format: vk::Format,
        model: &mut yumeri_live2d::Live2DModel,
    ) -> Result<Self> {
        let texture_images = load_model_textures(gpu, model)?;
        Self::new_with_options(
            gpu,
            swapchain_format,
            model,
            texture_images,
            RendererOptions::default(),
        )
    }

    pub fn new(
        gpu: &GpuContext,
        swapchain_format: vk::Format,
        model: &mut yumeri_live2d::Live2DModel,
        texture_images: Vec<Image>,
    ) -> Result<Self> {
        Self::new_with_options(
            gpu,
            swapchain_format,
            model,
            texture_images,
            RendererOptions::default(),
        )
    }

    pub fn new_with_options(
        gpu: &GpuContext,
        swapchain_format: vk::Format,
        model: &mut yumeri_live2d::Live2DModel,
        texture_images: Vec<Image>,
        options: RendererOptions,
    ) -> Result<Self> {
        let device = gpu.ash_device();

        let mut clip_manager = ClippingManager::new(model.core_model_mut())?;
        clip_manager.mask_buffer_size = [MASK_ATLAS_SIZE as f32; 2];
        clip_manager.setup_clipping(
            model.core_model_mut(),
            false,
            options.use_high_precision_mask,
            1,
        )?;

        // Create mask image
        let mask_image = Image::new(
            gpu,
            MASK_ATLAS_SIZE,
            MASK_ATLAS_SIZE,
            vk::Format::R8G8B8A8_UNORM,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            MemoryLocation::GpuOnly,
        )?;

        // Create samplers
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        // Descriptor set layouts
        let uniform_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT);
        let uniform_dsl = unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&uniform_binding)),
                None,
            )?
        };

        let tex_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);
        // Shared layout for both texture (set 1) and mask (set 2) descriptor sets
        let texture_dsl = unsafe {
            device.create_descriptor_set_layout(
                &vk::DescriptorSetLayoutCreateInfo::default()
                    .bindings(std::slice::from_ref(&tex_binding)),
                None,
            )?
        };
        let mask_dsl = texture_dsl;

        // Descriptor pool
        let max_texture_sets = texture_images.len().max(1) as u32;
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
                .descriptor_count(1),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(max_texture_sets + 1), // textures + mask
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(max_texture_sets + 2) // texture sets + uniform set + mask set
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        // Allocate descriptor sets
        let uniform_set = {
            let layouts = [uniform_dsl];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);
            unsafe { device.allocate_descriptor_sets(&alloc_info) }?[0]
        };

        let mask_set = {
            let layouts = [mask_dsl];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);
            unsafe { device.allocate_descriptor_sets(&alloc_info) }?[0]
        };

        let texture_sets = if !texture_images.is_empty() {
            let layouts = vec![texture_dsl; texture_images.len()];
            let alloc_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&layouts);
            unsafe { device.allocate_descriptor_sets(&alloc_info)? }
        } else {
            Vec::new()
        };

        // Create uniform buffer
        let drawables = model.drawables().map_err(RendererError::from)?;
        let drawable_count = drawables.len() as u64;
        drop(drawables);
        let uniform_capacity = (drawable_count * 3) + 256;
        let uniform_buffer = Buffer::new(
            gpu,
            UNIFORM_STRIDE * uniform_capacity,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
        )?;

        // Write uniform descriptor set
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(uniform_buffer.raw())
            .offset(0)
            .range(UNIFORM_SIZE);
        let uniform_write = vk::WriteDescriptorSet::default()
            .dst_set(uniform_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(std::slice::from_ref(&buffer_info));
        unsafe { device.update_descriptor_sets(&[uniform_write], &[]) };

        // Write mask descriptor set
        let mask_img_info = vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(mask_image.view())
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
        let mask_write = vk::WriteDescriptorSet::default()
            .dst_set(mask_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&mask_img_info));
        unsafe { device.update_descriptor_sets(&[mask_write], &[]) };

        // Write texture descriptor sets
        for (i, tex) in texture_images.iter().enumerate() {
            let img_info = vk::DescriptorImageInfo::default()
                .sampler(sampler)
                .image_view(tex.view())
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            let write = vk::WriteDescriptorSet::default()
                .dst_set(texture_sets[i])
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&img_info));
            unsafe { device.update_descriptor_sets(&[write], &[]) };
        }

        // Create pipelines
        let pipelines =
            create_all_pipelines(device, swapchain_format, uniform_dsl, texture_dsl, mask_dsl)?;

        // Create mesh buffers
        let drawables = model.drawables().map_err(RendererError::from)?;
        let mut meshes = Vec::with_capacity(drawables.len());
        for _ in 0..drawables.len() {
            meshes.push(create_empty_mesh(gpu)?);
        }
        drop(drawables);

        let mut renderer = Self {
            mask_image,
            sampler,
            mask_descriptor_set: mask_set,
            sampler_descriptor_set_layout: texture_dsl,
            texture_descriptor_sets: texture_sets,
            texture_images,
            uniform_buffer,
            uniform_capacity,
            uniform_descriptor_set_layout: uniform_dsl,
            uniform_descriptor_set: uniform_set,
            descriptor_pool,
            pipelines,
            meshes,
            sorted_drawables: Vec::new(),
            clip_manager,
            options,
            device: device.clone(),
        };

        renderer.sync_meshes(gpu, model)?;
        Ok(renderer)
    }

    pub fn set_use_high_precision_mask(&mut self, value: bool) {
        self.options.use_high_precision_mask = value;
    }

    pub fn register_pass(
        &mut self,
        gpu: &GpuContext,
        builder: &mut RenderGraphBuilder,
        backbuffer: ResourceId,
        model: &mut yumeri_live2d::Live2DModel,
        mvp: Mat4,
    ) -> Result<()> {
        let model_opacity = model.model_opacity().clamp(0.0, 1.0);
        let supported_low_precision = self.clip_manager.contexts().len() <= 36;
        let use_high_precision_mask =
            self.options.use_high_precision_mask || !supported_low_precision;

        self.clip_manager
            .setup_clipping(model.core_model_mut(), false, use_high_precision_mask, 1)?;
        self.sync_meshes(gpu, model)?;

        let drawables = model.drawables().map_err(RendererError::from)?;

        // Sort by render order
        self.sorted_drawables.resize(drawables.len(), 0);
        for (idx, slot) in self.sorted_drawables.iter_mut().enumerate() {
            *slot = idx;
        }
        self.sorted_drawables
            .sort_by_key(|&i| drawables.render_orders()[i]);

        // Prepare uniform data
        let mask_uniform_count = self
            .clip_manager
            .contexts()
            .iter()
            .filter(|c| c.is_using)
            .map(|c| c.clipping_id_list.len() as u64)
            .sum::<u64>();
        let required_uniforms = (drawables.len() as u64) + mask_uniform_count;

        if required_uniforms > self.uniform_capacity {
            self.grow_uniform_buffer(gpu, required_uniforms)?;
        }

        let premultiplied_alpha = self.options.premultiplied_alpha;
        let mvp_cols = mvp.to_cols_array_2d();
        let identity_cols = Mat4::IDENTITY.to_cols_array_2d();

        let uniform_data = self.uniform_buffer.mapped_slice_mut().unwrap();
        let mut uniforms_written = 0u64;

        // Build drawable context cache
        let default_channel = [1.0f32, 0.0, 0.0, 0.0];
        let contexts = self.clip_manager.contexts();

        // Write mask uniforms for low-precision path
        if !use_high_precision_mask {
            for ctx in contexts {
                if !ctx.is_using {
                    continue;
                }
                let channel_flag = channel_flag_as_color(ctx.layout_channel_index);
                let rect = RectF {
                    x: ctx.layout_bounds.x * 2.0 - 1.0,
                    y: ctx.layout_bounds.y * 2.0 - 1.0,
                    width: ctx.layout_bounds.width * 2.0,
                    height: ctx.layout_bounds.height * 2.0,
                };
                let base_color = [rect.x, rect.y, rect.right(), rect.bottom()];
                let clip_matrix = ctx.matrix_for_mask.to_cols_array_2d();

                for &clip_draw_index in &ctx.clipping_id_list {
                    if uniforms_written >= self.uniform_capacity {
                        break;
                    }
                    let multiply = drawables.multiply_color(clip_draw_index);
                    let screen = drawables.screen_color(clip_draw_index);
                    let u = Uniforms {
                        mvp: identity_cols,
                        clip: clip_matrix,
                        base_color,
                        multiply_color: [multiply.X, multiply.Y, multiply.Z, multiply.W],
                        screen_color: [screen.X, screen.Y, screen.Z, screen.W],
                        channel_flag,
                        flags: [if premultiplied_alpha { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
                    };
                    write_uniform(uniform_data, uniforms_written, &u);
                    uniforms_written += 1;
                }
            }
        }
        let mask_uniforms_end = uniforms_written;

        // Write draw uniforms
        for &drawable_index in &self.sorted_drawables {
            let dyn_flags = drawables.dynamic_flags()[drawable_index];
            if (dyn_flags & (core::sys::csmIsVisible as u8)) == 0 {
                continue;
            }
            if self.meshes[drawable_index].index_count == 0 {
                continue;
            }
            if uniforms_written >= self.uniform_capacity {
                break;
            }

            let ctx_idx = self.clip_manager.context_index_for_drawable(drawable_index);
            let masked = drawables.mask_counts()[drawable_index] > 0;
            let const_flags = drawables.constant_flags()[drawable_index] as u32;
            let inverted = (const_flags & (core::sys::csmIsInvertedMask as u32)) != 0;

            let opacity = drawables.opacities()[drawable_index] * model_opacity;
            let multiply = drawables.multiply_color(drawable_index);
            let screen = drawables.screen_color(drawable_index);
            let clip = if let Some(ci) = ctx_idx {
                contexts[ci].matrix_for_draw.to_cols_array_2d()
            } else {
                identity_cols
            };
            let channel_flag = ctx_idx
                .map(|ci| channel_flag_as_color(contexts[ci].layout_channel_index))
                .unwrap_or(default_channel);

            let draw_mode = if !masked {
                0.0
            } else if !inverted {
                1.0
            } else {
                2.0
            };

            let u = Uniforms {
                mvp: mvp_cols,
                clip,
                base_color: [1.0, 1.0, 1.0, opacity],
                multiply_color: [multiply.X, multiply.Y, multiply.Z, multiply.W],
                screen_color: [screen.X, screen.Y, screen.Z, screen.W],
                channel_flag,
                flags: [if premultiplied_alpha { 1.0 } else { 0.0 }, draw_mode, 0.0, 0.0],
            };
            write_uniform(uniform_data, uniforms_written, &u);
            uniforms_written += 1;
        }

        // Capture state for the closure
        let device = self.device.clone();
        let mask_image_raw = self.mask_image.raw();
        let mask_image_view = self.mask_image.view();
        let mask_ds = self.mask_descriptor_set;
        let uniform_ds = self.uniform_descriptor_set;
        let sorted = self.sorted_drawables.clone();
        let mesh_infos: Vec<_> = self
            .meshes
            .iter()
            .map(|m| (m.vertex.raw(), m.index.raw(), m.index_count))
            .collect();

        let drawable_infos: Vec<_> = (0..drawables.len())
            .map(|i| {
                let dyn_f = drawables.dynamic_flags()[i];
                let const_f = drawables.constant_flags()[i] as u32;
                let tex_idx = drawables.texture_indices()[i].max(0) as usize;
                let mask_count = drawables.mask_counts()[i];
                let ctx_idx = self.clip_manager.context_index_for_drawable(i);
                (dyn_f, const_f, tex_idx, mask_count, ctx_idx)
            })
            .collect();
        drop(drawables);

        let tex_sets = self.texture_descriptor_sets.clone();
        let fallback_tex_set = if tex_sets.is_empty() {
            vk::DescriptorSet::null()
        } else {
            tex_sets[0]
        };

        let mask_pipelines = [
            self.pipelines.mask_setup_cull.pipeline,
            self.pipelines.mask_setup_no_cull.pipeline,
        ];
        let mask_pl = self.pipelines.mask_setup_cull.pipeline_layout;

        let draw_pipelines = [
            self.pipelines.normal_cull.pipeline,
            self.pipelines.normal_no_cull.pipeline,
            self.pipelines.add_cull.pipeline,
            self.pipelines.add_no_cull.pipeline,
            self.pipelines.mult_cull.pipeline,
            self.pipelines.mult_no_cull.pipeline,
        ];
        let draw_pl = self.pipelines.normal_cull.pipeline_layout;

        let clip_contexts: Vec<_> = self
            .clip_manager
            .contexts()
            .iter()
            .map(|c| {
                (
                    c.is_using,
                    c.clipping_id_list.clone(),
                    c.layout_channel_index,
                    c.layout_bounds,
                )
            })
            .collect();

        let hp_mask = use_high_precision_mask;
        let reset_flags = self.options.reset_dynamic_flags;

        builder.add_pass("live2d", |pb| {
            pb.write(backbuffer);

            move |ctx| {
                let device = &device;
                let cmd = ctx.command_buffer();
                let extent = ctx.render_area();

                // End the current rendering pass to draw masks
                unsafe { device.cmd_end_rendering(cmd) };

                if !hp_mask {
                    // Low-precision: draw all masks at once
                    draw_mask_atlas(
                        device,
                        cmd,
                        mask_image_raw,
                        mask_image_view,
                        &mask_pipelines,
                        mask_pl,
                        uniform_ds,
                        &tex_sets,
                        fallback_tex_set,
                        &mesh_infos,
                        &drawable_infos,
                        &clip_contexts,
                    );
                } else {
                    // High-precision path not yet implemented; clear mask to white
                    // so masked drawables render as if unmasked rather than corrupted
                    log::warn!("Live2D high-precision mask path not yet implemented, falling back to unmasked rendering");
                    clear_mask_atlas(device, cmd, mask_image_raw, mask_image_view);
                }

                // Transition mask to shader read
                transition_mask_to_shader_read(device, cmd, mask_image_raw);

                // Resume main rendering
                let color_attachment = vk::RenderingAttachmentInfo::default()
                    .image_view(ctx.color_attachment())
                    .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::LOAD)
                    .store_op(vk::AttachmentStoreOp::STORE);
                let rendering_info = vk::RenderingInfo::default()
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    })
                    .layer_count(1)
                    .color_attachments(std::slice::from_ref(&color_attachment));
                unsafe { device.cmd_begin_rendering(cmd, &rendering_info) };

                // Set viewport and scissor
                let viewport = vk::Viewport {
                    x: 0.0,
                    y: extent.height as f32,
                    width: extent.width as f32,
                    height: -(extent.height as f32),
                    min_depth: 0.0,
                    max_depth: 1.0,
                };
                let scissor = vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                };
                unsafe {
                    device.cmd_set_viewport(cmd, 0, &[viewport]);
                    device.cmd_set_scissor(cmd, 0, &[scissor]);
                }

                // Draw all visible drawables
                // Bind mask descriptor set once (shared by all drawables)
                unsafe {
                    device.cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        draw_pl,
                        2,
                        &[mask_ds],
                        &[],
                    );
                }

                let mut draw_uniform_idx = mask_uniforms_end;
                let mut last_pipeline = vk::Pipeline::null();
                let mut last_tex_idx: Option<usize> = None;

                for &drawable_index in &sorted {
                    let (dyn_f, const_f, tex_idx, _mask_count, _ctx_idx) =
                        drawable_infos[drawable_index];
                    if (dyn_f & (core::sys::csmIsVisible as u8)) == 0 {
                        continue;
                    }
                    let (vb, ib, ic) = mesh_infos[drawable_index];
                    if ic == 0 {
                        continue;
                    }
                    if draw_uniform_idx >= uniforms_written {
                        break;
                    }

                    let cull = (const_f & (core::sys::csmIsDoubleSided as u32)) == 0;
                    let blend = if (const_f & (core::sys::csmBlendAdditive as u32)) != 0 {
                        Live2DBlendMode::Add
                    } else if (const_f & (core::sys::csmBlendMultiplicative as u32)) != 0 {
                        Live2DBlendMode::Multiply
                    } else {
                        Live2DBlendMode::Normal
                    };

                    let pipeline = select_draw_pipeline(&draw_pipelines, blend, cull);

                    if pipeline != last_pipeline {
                        unsafe { device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline) };
                        last_pipeline = pipeline;
                    }

                    let offset = (draw_uniform_idx * UNIFORM_STRIDE) as u32;
                    unsafe {
                        device.cmd_bind_descriptor_sets(
                            cmd,
                            vk::PipelineBindPoint::GRAPHICS,
                            draw_pl,
                            0,
                            &[uniform_ds],
                            &[offset],
                        );
                    }

                    if last_tex_idx != Some(tex_idx) {
                        let tex_ds = tex_sets.get(tex_idx).copied().unwrap_or(fallback_tex_set);
                        unsafe {
                            device.cmd_bind_descriptor_sets(
                                cmd,
                                vk::PipelineBindPoint::GRAPHICS,
                                draw_pl,
                                1,
                                &[tex_ds],
                                &[],
                            );
                        }
                        last_tex_idx = Some(tex_idx);
                    }

                    unsafe {
                        device.cmd_bind_vertex_buffers(cmd, 0, &[vb], &[0]);
                        device.cmd_bind_index_buffer(cmd, ib, 0, vk::IndexType::UINT16);
                        device.cmd_draw_indexed(cmd, ic as u32, 1, 0, 0, 0);
                    }

                    draw_uniform_idx += 1;
                }

                let _ = reset_flags; // TODO: reset model dynamic flags through callback
            }
        });

        Ok(())
    }

    fn sync_meshes(
        &mut self,
        gpu: &GpuContext,
        model: &mut yumeri_live2d::Live2DModel,
    ) -> Result<()> {
        let drawables = model.drawables().map_err(RendererError::from)?;

        if self.meshes.len() != drawables.len() {
            self.meshes.clear();
            for _ in 0..drawables.len() {
                self.meshes.push(create_empty_mesh(gpu)?);
            }
        }

        let mut needs_wait = false;
        for drawable_index in 0..drawables.len() {
            let positions = drawables.vertex_positions(drawable_index);
            let indices = drawables.indices(drawable_index);
            let vertex_count = positions.len();
            let index_count = indices.len();

            if vertex_count > self.meshes[drawable_index].vertex_capacity
                || index_count > self.meshes[drawable_index].index_capacity
            {
                needs_wait = true;
                break;
            }
        }

        // Wait for in-flight frames before replacing any buffer the GPU may still reference.
        if needs_wait {
            unsafe { gpu.ash_device().device_wait_idle()? };
        }

        for drawable_index in 0..drawables.len() {
            let positions = drawables.vertex_positions(drawable_index);
            let uvs = drawables.vertex_uvs(drawable_index);
            let indices = drawables.indices(drawable_index);

            let vertex_count = positions.len();
            let index_count = indices.len();

            let mesh = &mut self.meshes[drawable_index];
            mesh.index_count = index_count;

            if vertex_count > mesh.vertex_capacity {
                mesh.vertex = Buffer::new(
                    gpu,
                    (vertex_count * std::mem::size_of::<Vertex>()) as u64,
                    vk::BufferUsageFlags::VERTEX_BUFFER,
                    MemoryLocation::CpuToGpu,
                )?;
                mesh.vertex_capacity = vertex_count;
                mesh.vertex_uploaded = false;
            }
            if index_count > mesh.index_capacity {
                mesh.index = Buffer::new(
                    gpu,
                    (index_count * std::mem::size_of::<u16>()) as u64,
                    vk::BufferUsageFlags::INDEX_BUFFER,
                    MemoryLocation::CpuToGpu,
                )?;
                mesh.index_capacity = index_count;
                mesh.index_uploaded = false;
            }

            let dyn_flags = drawables.dynamic_flags()[drawable_index];
            let vertex_changed =
                (dyn_flags & (core::sys::csmVertexPositionsDidChange as u8)) != 0;

            if vertex_count > 0 && (!mesh.vertex_uploaded || vertex_changed) {
                if let Some(mapped) = mesh.vertex.mapped_slice_mut() {
                    let dst = bytemuck::cast_slice_mut::<u8, Vertex>(
                        &mut mapped[..vertex_count * std::mem::size_of::<Vertex>()],
                    );
                    for i in 0..vertex_count {
                        dst[i] = Vertex {
                            position: [positions[i].X, positions[i].Y],
                            uv: [uvs[i].X, uvs[i].Y],
                        };
                    }
                }
                mesh.vertex_uploaded = true;
            }
            if index_count > 0 && !mesh.index_uploaded {
                if let Some(mapped) = mesh.index.mapped_slice_mut() {
                    let dst = &mut mapped[..index_count * std::mem::size_of::<u16>()];
                    dst.copy_from_slice(bytemuck::cast_slice(indices));
                }
                mesh.index_uploaded = true;
            }
        }

        Ok(())
    }

    fn grow_uniform_buffer(&mut self, gpu: &GpuContext, required: u64) -> Result<()> {
        // Wait for all in-flight frames to finish before replacing the buffer,
        // since previous frames' command buffers may still reference the old one.
        unsafe { gpu.ash_device().device_wait_idle()? };

        let new_capacity = required.next_power_of_two().max(self.uniform_capacity);
        self.uniform_buffer = Buffer::new(
            gpu,
            UNIFORM_STRIDE * new_capacity,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            MemoryLocation::CpuToGpu,
        )?;
        self.uniform_capacity = new_capacity;

        // Rebind the new buffer to the descriptor set
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(self.uniform_buffer.raw())
            .offset(0)
            .range(UNIFORM_SIZE);
        let write = vk::WriteDescriptorSet::default()
            .dst_set(self.uniform_descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC)
            .buffer_info(std::slice::from_ref(&buffer_info));
        unsafe { self.device.update_descriptor_sets(&[write], &[]) };

        Ok(())
    }

    pub fn destroy(self, gpu: &GpuContext) {
        let device = gpu.ash_device();
        unsafe {
            let _ = device.device_wait_idle();
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.uniform_descriptor_set_layout, None);
            device.destroy_descriptor_set_layout(self.sampler_descriptor_set_layout, None);
            device.destroy_sampler(self.sampler, None);
        }
        destroy_pipelines(device, &self.pipelines);
        // Buffer, Image fields are cleaned up by Drop when self is consumed
    }
}

impl From<core::Error> for RendererError {
    fn from(e: core::Error) -> Self {
        RendererError::Shader(format!("Live2D core error: {e}"))
    }
}

fn write_uniform(data: &mut [u8], index: u64, uniforms: &Uniforms) {
    let offset = (index * UNIFORM_STRIDE) as usize;
    let bytes = bytemuck::bytes_of(uniforms);
    data[offset..(offset + bytes.len())].copy_from_slice(bytes);
}

fn create_empty_mesh(gpu: &GpuContext) -> Result<MeshBuffers> {
    Ok(MeshBuffers {
        vertex: Buffer::new(
            gpu,
            64,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            MemoryLocation::CpuToGpu,
        )?,
        index: Buffer::new(
            gpu,
            64,
            vk::BufferUsageFlags::INDEX_BUFFER,
            MemoryLocation::CpuToGpu,
        )?,
        vertex_capacity: 0,
        index_capacity: 0,
        index_count: 0,
        vertex_uploaded: false,
        index_uploaded: false,
    })
}

fn vertex_input() -> (
    Vec<vk::VertexInputBindingDescription>,
    Vec<vk::VertexInputAttributeDescription>,
) {
    let binding = vk::VertexInputBindingDescription::default()
        .binding(0)
        .stride(std::mem::size_of::<Vertex>() as u32)
        .input_rate(vk::VertexInputRate::VERTEX);
    let attrs = vec![
        vk::VertexInputAttributeDescription::default()
            .location(0)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(0),
        vk::VertexInputAttributeDescription::default()
            .location(1)
            .binding(0)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(8),
    ];
    (vec![binding], attrs)
}

fn create_all_pipelines(
    device: &ash::Device,
    swapchain_format: vk::Format,
    uniform_dsl: vk::DescriptorSetLayout,
    texture_dsl: vk::DescriptorSetLayout,
    mask_dsl: vk::DescriptorSetLayout,
) -> Result<Pipelines> {
    let vert_spv = include_bytes!(concat!(env!("OUT_DIR"), "/live2d_live2d.vert.spv"));
    let frag_spv = include_bytes!(concat!(env!("OUT_DIR"), "/live2d_live2d.frag.spv"));
    let mask_vert_spv = include_bytes!(concat!(env!("OUT_DIR"), "/live2d_live2d_mask.vert.spv"));
    let mask_frag_spv = include_bytes!(concat!(env!("OUT_DIR"), "/live2d_live2d_mask.frag.spv"));

    let (vb, va) = vertex_input();

    let mask_blend = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ZERO)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_COLOR)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ZERO)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let normal_blend = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let add_blend = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ONE)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ZERO)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let mult_blend = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::DST_COLOR)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ZERO)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);

    let build = |vert: &[u8],
                 frag: &[u8],
                 format: vk::Format,
                 blend: vk::PipelineColorBlendAttachmentState,
                 cull: vk::CullModeFlags| {
        PipelineBuilder::new(device, format, vert, frag)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .vertex_input(vb.clone(), va.clone())
            .custom_blend(blend)
            .cull_mode(cull)
            .external_set_layout(uniform_dsl)
            .external_set_layout(texture_dsl)
            .external_set_layout(mask_dsl)
            .build()
    };

    let mask_format = vk::Format::R8G8B8A8_UNORM;
    let cull_back = vk::CullModeFlags::BACK;
    let no_cull = vk::CullModeFlags::NONE;

    // Build mask pipelines (only need texture_dsl for mask setup, not mask_dsl)
    let build_mask = |cull: vk::CullModeFlags| {
        PipelineBuilder::new(device, mask_format, mask_vert_spv, mask_frag_spv)
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .vertex_input(vb.clone(), va.clone())
            .custom_blend(mask_blend)
            .cull_mode(cull)
            .external_set_layout(uniform_dsl)
            .external_set_layout(texture_dsl)
            .build()
    };

    Ok(Pipelines {
        mask_setup_cull: build_mask(cull_back)?,
        mask_setup_no_cull: build_mask(no_cull)?,

        normal_cull: build(vert_spv, frag_spv, swapchain_format, normal_blend, cull_back)?,
        normal_no_cull: build(vert_spv, frag_spv, swapchain_format, normal_blend, no_cull)?,
        add_cull: build(vert_spv, frag_spv, swapchain_format, add_blend, cull_back)?,
        add_no_cull: build(vert_spv, frag_spv, swapchain_format, add_blend, no_cull)?,
        mult_cull: build(vert_spv, frag_spv, swapchain_format, mult_blend, cull_back)?,
        mult_no_cull: build(vert_spv, frag_spv, swapchain_format, mult_blend, no_cull)?,
    })
}

fn select_draw_pipeline(
    pipelines: &[vk::Pipeline; 6],
    blend: Live2DBlendMode,
    cull: bool,
) -> vk::Pipeline {
    let blend_offset = match blend {
        Live2DBlendMode::Normal => 0,
        Live2DBlendMode::Add => 2,
        Live2DBlendMode::Multiply => 4,
    };
    let cull_offset = if cull { 0 } else { 1 };
    pipelines[blend_offset + cull_offset]
}

fn destroy_pipelines(device: &ash::Device, p: &Pipelines) {
    p.mask_setup_cull.destroy(device);
    p.mask_setup_no_cull.destroy(device);
    p.normal_cull.destroy(device);
    p.normal_no_cull.destroy(device);
    p.add_cull.destroy(device);
    p.add_no_cull.destroy(device);
    p.mult_cull.destroy(device);
    p.mult_no_cull.destroy(device);
}

fn draw_mask_atlas(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    mask_image: vk::Image,
    mask_view: vk::ImageView,
    mask_pipelines: &[vk::Pipeline; 2],
    pipeline_layout: vk::PipelineLayout,
    uniform_ds: vk::DescriptorSet,
    tex_sets: &[vk::DescriptorSet],
    fallback_tex_set: vk::DescriptorSet,
    mesh_infos: &[(vk::Buffer, vk::Buffer, usize)],
    drawable_infos: &[(u8, u32, usize, i32, Option<usize>)],
    clip_contexts: &[(bool, Vec<usize>, usize, RectF)],
) {
    unsafe {
        // Transition mask image to color attachment
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(mask_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let dep_info = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier));
        device.cmd_pipeline_barrier2(cmd, &dep_info);

        // Begin mask rendering
        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        };
        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(mask_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_value);
        let mask_extent = vk::Extent2D {
            width: MASK_ATLAS_SIZE,
            height: MASK_ATLAS_SIZE,
        };
        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: mask_extent,
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment));
        device.cmd_begin_rendering(cmd, &rendering_info);

        let mask_size_f = MASK_ATLAS_SIZE as f32;
        let viewport = vk::Viewport {
            x: 0.0,
            y: mask_size_f,
            width: mask_size_f,
            height: -mask_size_f,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: mask_extent,
        };
        device.cmd_set_viewport(cmd, 0, &[viewport]);
        device.cmd_set_scissor(cmd, 0, &[scissor]);

        let mut uniform_idx = 0u64;
        let mut last_pipeline = vk::Pipeline::null();
        let mut last_tex_idx: Option<usize> = None;

        for (is_using, clip_ids, _channel_idx, _bounds) in clip_contexts {
            if !is_using {
                continue;
            }

            for &clip_draw_index in clip_ids {
                let (_dyn_f, const_f, tex_idx, _mask_count, _ctx_idx) =
                    drawable_infos[clip_draw_index];
                let (vb, ib, ic) = mesh_infos[clip_draw_index];
                if ic == 0 {
                    uniform_idx += 1;
                    continue;
                }

                let cull = (const_f & (core::sys::csmIsDoubleSided as u32)) == 0;
                let pipeline = if cull {
                    mask_pipelines[0]
                } else {
                    mask_pipelines[1]
                };

                if pipeline != last_pipeline {
                    device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);
                    last_pipeline = pipeline;
                }

                let offset = (uniform_idx * UNIFORM_STRIDE) as u32;
                device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    &[uniform_ds],
                    &[offset],
                );

                if last_tex_idx != Some(tex_idx) {
                    let tex_ds = tex_sets.get(tex_idx).copied().unwrap_or(fallback_tex_set);
                    device.cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        1,
                        &[tex_ds],
                        &[],
                    );
                    last_tex_idx = Some(tex_idx);
                }

                device.cmd_bind_vertex_buffers(cmd, 0, &[vb], &[0]);
                device.cmd_bind_index_buffer(cmd, ib, 0, vk::IndexType::UINT16);
                device.cmd_draw_indexed(cmd, ic as u32, 1, 0, 0, 0);

                uniform_idx += 1;
            }
        }

        device.cmd_end_rendering(cmd);
    }
}

fn transition_mask_to_shader_read(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    mask_image: vk::Image,
) {
    let barrier = vk::ImageMemoryBarrier2::default()
        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
        .dst_stage_mask(vk::PipelineStageFlags2::FRAGMENT_SHADER)
        .dst_access_mask(vk::AccessFlags2::SHADER_SAMPLED_READ)
        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image(mask_image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });
    let dep_info =
        vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
    unsafe { device.cmd_pipeline_barrier2(cmd, &dep_info) };
}

fn clear_mask_atlas(
    device: &ash::Device,
    cmd: vk::CommandBuffer,
    mask_image: vk::Image,
    mask_view: vk::ImageView,
) {
    unsafe {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .image(mask_image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        let dep_info =
            vk::DependencyInfo::default().image_memory_barriers(std::slice::from_ref(&barrier));
        device.cmd_pipeline_barrier2(cmd, &dep_info);

        let clear_value = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        };
        let color_attachment = vk::RenderingAttachmentInfo::default()
            .image_view(mask_view)
            .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .clear_value(clear_value);
        let mask_extent = vk::Extent2D {
            width: MASK_ATLAS_SIZE,
            height: MASK_ATLAS_SIZE,
        };
        let rendering_info = vk::RenderingInfo::default()
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: mask_extent,
            })
            .layer_count(1)
            .color_attachments(std::slice::from_ref(&color_attachment));
        device.cmd_begin_rendering(cmd, &rendering_info);
        device.cmd_end_rendering(cmd);
    }
}

fn load_model_textures(
    gpu: &GpuContext,
    model: &yumeri_live2d::Live2DModel,
) -> Result<Vec<Image>> {
    let paths = model.texture_paths();
    let mut images = Vec::with_capacity(paths.len());
    for path in paths {
        let img = yumeri_image::Image::load(path).map_err(|e| {
            RendererError::Texture(format!("failed to load Live2D texture {}: {e}", path.display()))
        })?;
        let gpu_image =
            crate::texture::store::upload_image_to_gpu(gpu, img.width(), img.height(), img.data())?;
        images.push(gpu_image);
    }
    Ok(images)
}
