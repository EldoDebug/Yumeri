use glam::{Mat4, Vec3};
use yumeri_live2d::core;

#[derive(Debug, Clone, Copy, Default)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl RectF {
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn expand(&mut self, dx: f32, dy: f32) {
        self.x -= dx;
        self.y -= dy;
        self.width += dx * 2.0;
        self.height += dy * 2.0;
    }
}

#[derive(Debug)]
pub struct ClippingContext {
    pub is_using: bool,
    pub clipping_id_list: Vec<usize>,
    pub layout_channel_index: usize,
    pub layout_bounds: RectF,
    pub all_clipped_draw_rect: RectF,
    pub matrix_for_mask: Mat4,
    pub matrix_for_draw: Mat4,
    pub clipped_drawable_indices: Vec<usize>,
    pub buffer_index: usize,
}

#[derive(Debug)]
pub struct ClippingManager {
    pub mask_buffer_size: [f32; 2],
    contexts: Vec<ClippingContext>,
    context_for_draw: Vec<Option<usize>>,
}

impl Default for ClippingManager {
    fn default() -> Self {
        Self {
            mask_buffer_size: [256.0, 256.0],
            contexts: Vec::new(),
            context_for_draw: Vec::new(),
        }
    }
}

impl ClippingManager {
    pub fn new(model: &mut core::Model) -> Result<Self, core::Error> {
        let drawables = model.drawables()?;
        let mut contexts: Vec<ClippingContext> = Vec::new();
        let mut context_for_draw = vec![None; drawables.len()];

        for drawable_index in 0..drawables.len() {
            let Some(masks) = drawables.masks(drawable_index) else {
                continue;
            };
            let masks_usize = masks
                .iter()
                .copied()
                .filter(|&m| m >= 0)
                .map(|m| m as usize)
                .collect::<Vec<_>>();
            if masks_usize.is_empty() {
                continue;
            }

            let existing = contexts
                .iter()
                .position(|c| same_mask_set(&c.clipping_id_list, &masks_usize));
            let ctx_idx = if let Some(idx) = existing {
                idx
            } else {
                contexts.push(ClippingContext {
                    is_using: false,
                    clipping_id_list: masks_usize.clone(),
                    layout_channel_index: 0,
                    layout_bounds: RectF {
                        x: 0.0,
                        y: 0.0,
                        width: 1.0,
                        height: 1.0,
                    },
                    all_clipped_draw_rect: RectF::default(),
                    matrix_for_mask: Mat4::IDENTITY,
                    matrix_for_draw: Mat4::IDENTITY,
                    clipped_drawable_indices: Vec::new(),
                    buffer_index: 0,
                });
                contexts.len() - 1
            };

            contexts[ctx_idx]
                .clipped_drawable_indices
                .push(drawable_index);
            context_for_draw[drawable_index] = Some(ctx_idx);
        }

        Ok(Self {
            contexts,
            context_for_draw,
            ..Default::default()
        })
    }

    pub fn setup_clipping(
        &mut self,
        model: &mut core::Model,
        is_right_handed: bool,
        use_high_precision_mask: bool,
        render_texture_count: usize,
    ) -> Result<(), core::Error> {
        if use_high_precision_mask {
            self.setup_matrix_for_high_precision(model, is_right_handed)?;
        } else {
            self.setup_matrix_for_low_precision(model, is_right_handed, render_texture_count)?;
        }
        Ok(())
    }

    pub fn context_for_drawable(&self, drawable_index: usize) -> Option<&ClippingContext> {
        let idx = self
            .context_for_draw
            .get(drawable_index)
            .copied()
            .flatten()?;
        self.contexts.get(idx)
    }

    pub fn context_index_for_drawable(&self, drawable_index: usize) -> Option<usize> {
        self.context_for_draw.get(drawable_index).copied().flatten()
    }

    pub fn contexts(&self) -> &[ClippingContext] {
        &self.contexts
    }

    pub fn setup_matrix_for_low_precision(
        &mut self,
        model: &mut core::Model,
        is_right_handed: bool,
        render_texture_count: usize,
    ) -> Result<(), core::Error> {
        let drawables = model.drawables()?;

        let mut using_context_indices = Vec::new();
        for (ctx_idx, ctx) in self.contexts.iter_mut().enumerate() {
            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for &drawable_index in &ctx.clipped_drawable_indices {
                let verts = drawables.vertex_positions(drawable_index);
                for v in verts {
                    min_x = min_x.min(v.X);
                    min_y = min_y.min(v.Y);
                    max_x = max_x.max(v.X);
                    max_y = max_y.max(v.Y);
                }
            }

            if !min_x.is_finite() {
                ctx.is_using = false;
                ctx.all_clipped_draw_rect = RectF::default();
                ctx.layout_channel_index = 0;
                ctx.layout_bounds = RectF {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                };
                ctx.buffer_index = 0;
                ctx.matrix_for_mask = Mat4::IDENTITY;
                ctx.matrix_for_draw = Mat4::IDENTITY;
                continue;
            }

            ctx.is_using = true;
            ctx.all_clipped_draw_rect = RectF {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            };
            using_context_indices.push(ctx_idx);
        }

        if using_context_indices.is_empty() {
            return Ok(());
        }

        self.setup_layout_bounds(&using_context_indices, render_texture_count);

        for &ctx_idx in &using_context_indices {
            let ctx = &mut self.contexts[ctx_idx];
            let margin = 0.05;
            let mut tmp_bounds = ctx.all_clipped_draw_rect;
            tmp_bounds.expand(
                ctx.all_clipped_draw_rect.width * margin,
                ctx.all_clipped_draw_rect.height * margin,
            );

            let scale_x = if tmp_bounds.width.abs() <= f32::EPSILON {
                0.0
            } else {
                ctx.layout_bounds.width / tmp_bounds.width
            };
            let scale_y = if tmp_bounds.height.abs() <= f32::EPSILON {
                0.0
            } else {
                ctx.layout_bounds.height / tmp_bounds.height
            };

            let (m_mask, m_draw) = create_matrix_for_mask(
                is_right_handed,
                ctx.layout_bounds,
                tmp_bounds,
                scale_x,
                scale_y,
            );
            ctx.matrix_for_mask = m_mask;
            ctx.matrix_for_draw = m_draw;
        }

        Ok(())
    }

    pub fn setup_matrix_for_high_precision(
        &mut self,
        model: &mut core::Model,
        is_right_handed: bool,
    ) -> Result<(), core::Error> {
        let ppu = model.canvas_info().pixels_per_unit;
        let drawables = model.drawables()?;

        for ctx in &mut self.contexts {
            ctx.layout_channel_index = 0;
            ctx.layout_bounds = RectF {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            };
            ctx.buffer_index = 0;

            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for &drawable_index in &ctx.clipped_drawable_indices {
                let verts = drawables.vertex_positions(drawable_index);
                for v in verts {
                    min_x = min_x.min(v.X);
                    min_y = min_y.min(v.Y);
                    max_x = max_x.max(v.X);
                    max_y = max_y.max(v.Y);
                }
            }

            if !min_x.is_finite() {
                ctx.is_using = false;
                ctx.all_clipped_draw_rect = RectF::default();
                continue;
            }

            ctx.is_using = true;
            ctx.all_clipped_draw_rect = RectF {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            };

            let margin = 0.05;
            let mask_pixel_width = self.mask_buffer_size[0];
            let mask_pixel_height = self.mask_buffer_size[1];
            let physical_mask_width = ctx.layout_bounds.width * mask_pixel_width;
            let physical_mask_height = ctx.layout_bounds.height * mask_pixel_height;

            let mut tmp_bounds = ctx.all_clipped_draw_rect;
            let scale_x;
            let scale_y;

            if tmp_bounds.width * ppu > physical_mask_width {
                tmp_bounds.expand(ctx.all_clipped_draw_rect.width * margin, 0.0);
                scale_x = ctx.layout_bounds.width / tmp_bounds.width;
            } else {
                scale_x = ppu / physical_mask_width;
            }

            if tmp_bounds.height * ppu > physical_mask_height {
                tmp_bounds.expand(0.0, ctx.all_clipped_draw_rect.height * margin);
                scale_y = ctx.layout_bounds.height / tmp_bounds.height;
            } else {
                scale_y = ppu / physical_mask_height;
            }

            let (m_mask, m_draw) = create_matrix_for_mask(
                is_right_handed,
                ctx.layout_bounds,
                tmp_bounds,
                scale_x,
                scale_y,
            );
            ctx.matrix_for_mask = m_mask;
            ctx.matrix_for_draw = m_draw;
        }

        Ok(())
    }

    fn setup_layout_bounds(
        &mut self,
        using_context_indices: &[usize],
        render_texture_count: usize,
    ) {
        let render_texture_count = render_texture_count.max(1) as i32;
        let using_clip_count = using_context_indices.len() as i32;
        let use_clipping_mask_max_count = if render_texture_count <= 1 {
            36
        } else {
            32 * render_texture_count
        };

        if using_clip_count <= 0 || using_clip_count > use_clipping_mask_max_count {
            for ctx in &mut self.contexts {
                ctx.layout_channel_index = 0;
                ctx.layout_bounds = RectF {
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                    height: 1.0,
                };
                ctx.buffer_index = 0;
            }
            return;
        }

        let layout_count_max_value = if render_texture_count <= 1 { 9 } else { 8 };
        let count_per_sheet_div =
            (using_clip_count + render_texture_count - 1) / render_texture_count;
        let reduce_layout_texture_count = using_clip_count % render_texture_count;

        const COLOR_CHANNEL_COUNT: i32 = 4;
        let div_count = count_per_sheet_div / COLOR_CHANNEL_COUNT;
        let mod_count = count_per_sheet_div % COLOR_CHANNEL_COUNT;

        let mut cur_clip_index = 0usize;

        for render_texture_index in 0..render_texture_count {
            for channel_index in 0..COLOR_CHANNEL_COUNT {
                let mut layout_count = div_count + if channel_index < mod_count { 1 } else { 0 };

                let check_channel_index = mod_count + if div_count < 1 { -1 } else { 0 };
                if channel_index == check_channel_index && reduce_layout_texture_count > 0 {
                    if !(render_texture_index < reduce_layout_texture_count) {
                        layout_count -= 1;
                    }
                }

                match layout_count {
                    0 => {}
                    1 => {
                        let Some(&ctx_idx) = using_context_indices.get(cur_clip_index) else {
                            return;
                        };
                        cur_clip_index += 1;
                        let ctx = &mut self.contexts[ctx_idx];
                        ctx.layout_channel_index = channel_index as usize;
                        ctx.layout_bounds = RectF {
                            x: 0.0,
                            y: 0.0,
                            width: 1.0,
                            height: 1.0,
                        };
                        ctx.buffer_index = render_texture_index as usize;
                    }
                    2 => {
                        for i in 0..layout_count {
                            let Some(&ctx_idx) = using_context_indices.get(cur_clip_index) else {
                                return;
                            };
                            cur_clip_index += 1;
                            let xpos = i % 2;
                            let ctx = &mut self.contexts[ctx_idx];
                            ctx.layout_channel_index = channel_index as usize;
                            ctx.layout_bounds = RectF {
                                x: (xpos as f32) * 0.5,
                                y: 0.0,
                                width: 0.5,
                                height: 1.0,
                            };
                            ctx.buffer_index = render_texture_index as usize;
                        }
                    }
                    3 | 4 => {
                        for i in 0..layout_count {
                            let Some(&ctx_idx) = using_context_indices.get(cur_clip_index) else {
                                return;
                            };
                            cur_clip_index += 1;
                            let xpos = i % 2;
                            let ypos = i / 2;
                            let ctx = &mut self.contexts[ctx_idx];
                            ctx.layout_channel_index = channel_index as usize;
                            ctx.layout_bounds = RectF {
                                x: (xpos as f32) * 0.5,
                                y: (ypos as f32) * 0.5,
                                width: 0.5,
                                height: 0.5,
                            };
                            ctx.buffer_index = render_texture_index as usize;
                        }
                    }
                    _ if layout_count <= layout_count_max_value => {
                        for i in 0..layout_count {
                            let Some(&ctx_idx) = using_context_indices.get(cur_clip_index) else {
                                return;
                            };
                            cur_clip_index += 1;
                            let xpos = i % 3;
                            let ypos = i / 3;
                            let ctx = &mut self.contexts[ctx_idx];
                            ctx.layout_channel_index = channel_index as usize;
                            ctx.layout_bounds = RectF {
                                x: (xpos as f32) / 3.0,
                                y: (ypos as f32) / 3.0,
                                width: 1.0 / 3.0,
                                height: 1.0 / 3.0,
                            };
                            ctx.buffer_index = render_texture_index as usize;
                        }
                    }
                    _ => {
                        for ctx in &mut self.contexts {
                            ctx.layout_channel_index = 0;
                            ctx.layout_bounds = RectF {
                                x: 0.0,
                                y: 0.0,
                                width: 1.0,
                                height: 1.0,
                            };
                            ctx.buffer_index = 0;
                        }
                        return;
                    }
                }
            }
        }
    }
}

pub fn channel_flag_as_color(layout_channel_index: usize) -> [f32; 4] {
    let layout_channel_index = layout_channel_index.min(3);
    let mut out = [0.0; 4];
    out[layout_channel_index] = 1.0;
    out
}

fn same_mask_set(a: &[usize], b: &[usize]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().all(|id| b.iter().any(|other| other == id))
}

fn create_matrix_for_mask(
    is_right_handed: bool,
    layout: RectF,
    bounds_on_model: RectF,
    scale_x: f32,
    scale_y: f32,
) -> (Mat4, Mat4) {
    let to_clip = Mat4::from_translation(Vec3::new(-1.0, -1.0, 0.0))
        * Mat4::from_scale(Vec3::new(2.0, 2.0, 1.0));
    let view_to_layout = Mat4::from_translation(Vec3::new(layout.x, layout.y, 0.0))
        * Mat4::from_scale(Vec3::new(scale_x, scale_y, 1.0))
        * Mat4::from_translation(Vec3::new(-bounds_on_model.x, -bounds_on_model.y, 0.0));
    let m_mask = to_clip * view_to_layout;

    let sgn = if is_right_handed { -1.0 } else { 1.0 };
    let m_draw = Mat4::from_translation(Vec3::new(layout.x, layout.y * sgn, 0.0))
        * Mat4::from_scale(Vec3::new(scale_x, scale_y * sgn, 1.0))
        * Mat4::from_translation(Vec3::new(-bounds_on_model.x, -bounds_on_model.y, 0.0));

    (m_mask, m_draw)
}
