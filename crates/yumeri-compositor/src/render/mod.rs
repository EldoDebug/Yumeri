pub mod surface_texture;

use std::cell::RefCell;

use yumeri_renderer::{Rect, Texture, TextureId};
use yumeri_types::Color;
use yumeri_wm::{FocusStack, WindowId};

use crate::compositor::{CompositorState, ManagedWindow};

pub fn render_frame(state: &mut CompositorState) {
    let pending = std::mem::take(&mut state.pending_images);

    let old_textures: Vec<TextureId> = pending.iter().filter_map(|(wid, _)| {
        state.windows.get(*wid).and_then(|w| w.texture_id)
    }).collect();

    thread_local! {
        static NEW_TEXTURES: RefCell<Vec<(WindowId, TextureId)>> = RefCell::new(Vec::new());
    }
    NEW_TEXTURES.with(|c| c.borrow_mut().clear());

    let removals = std::mem::take(&mut state.pending_texture_removals);

    // Build draw list before the render call; new textures won't appear until next frame
    let window_draw_list: Vec<((i32, i32), (u32, u32), Option<TextureId>)> =
        state.focus_stack.iter_back_to_front()
            .filter_map(|wid| {
                let w = state.windows.get(wid)?;
                if !w.mapped { return None; }
                Some((w.position, w.size, w.texture_id))
            })
            .collect();

    let output_size = state.output_size;
    let layer_shell = &state.layer_shell;

    let result = state.render_state.render_frame(&state.gpu, &state.pool, |ctx| {
        // Phase 1: Texture management
        for tex_id in &removals {
            ctx.remove_texture(*tex_id);
        }
        for tex_id in &old_textures {
            ctx.remove_texture(*tex_id);
        }
        for (wid, image) in &pending {
            match ctx.create_texture(image) {
                Ok(tex_id) => {
                    NEW_TEXTURES.with(|c| c.borrow_mut().push((*wid, tex_id)));
                }
                Err(e) => {
                    log::error!("Failed to create texture for window: {e}");
                }
            }
        }

        // Phase 2: Drawing
        layer_shell.render_below_windows(ctx, output_size);

        for (pos, size, tex_id) in &window_draw_list {
            let x = pos.0 as f32;
            let y = pos.1 as f32;
            let w = size.0 as f32;
            let h = size.1 as f32;

            let cx = x + w / 2.0;
            let cy = y + h / 2.0;
            let tex = tex_id.map(Texture::new);
            ctx.draw_rect(Rect {
                position: [cx, cy],
                size: [w / 2.0, h / 2.0],
                color: Color::WHITE,
                texture: tex,
            });
        }

        layer_shell.render_above_windows(ctx, output_size);
    }, None, None);

    if let Err(e) = result {
        log::error!("render_frame failed: {e}");
    }

    // Update texture IDs after render_frame completes
    NEW_TEXTURES.with(|c| {
        for (wid, tex_id) in c.borrow().iter() {
            if let Some(w) = state.windows.get_mut(*wid) {
                w.texture_id = Some(*tex_id);
            }
        }
    });
}

pub fn hit_test_window(
    focus_stack: &FocusStack,
    windows: &slotmap::SlotMap<WindowId, ManagedWindow>,
    px: f64,
    py: f64,
) -> Option<WindowId> {
    for wid in focus_stack.iter_front_to_back() {
        let Some(w) = windows.get(wid) else { continue };
        if !w.mapped { continue; }

        let (wx, wy) = (w.position.0 as f64, w.position.1 as f64);
        let (ww, wh) = (w.size.0 as f64, w.size.1 as f64);

        if px >= wx && px <= wx + ww && py >= wy && py <= wy + wh {
            return Some(wid);
        }
    }
    None
}
