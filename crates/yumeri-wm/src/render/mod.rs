pub mod surface_texture;

use std::cell::RefCell;

use yumeri_renderer::{
    Color, Rect, Texture, TextureId,
};

use crate::compositor::CompositorState;
use crate::event_loop;
use crate::shell::focus::FocusStack;
use crate::shell::window::WindowId;

pub fn render_frame(state: &mut CompositorState) {
    let pending = event_loop::take_pending_images();

    // Collect old texture IDs for windows that will get new textures
    let old_textures: Vec<TextureId> = pending.iter().filter_map(|(wid, _)| {
        state.windows.get(*wid).and_then(|w| w.texture_id)
    }).collect();

    // Use thread-local to pass texture IDs out of the closure (avoids rustc ICE with mut captures)
    thread_local! {
        static NEW_TEXTURES: RefCell<Vec<(WindowId, TextureId)>> = RefCell::new(Vec::new());
    }
    NEW_TEXTURES.with(|c| c.borrow_mut().clear());

    // Textures queued for removal from destroyed windows
    let removals = std::mem::take(&mut state.pending_texture_removals);

    let result = state.render_state.render_frame(&state.gpu, |ctx| {
        // Destroy textures from removed windows
        for tex_id in &removals {
            ctx.remove_texture(*tex_id);
        }

        // Destroy old textures before uploading replacements
        for tex_id in &old_textures {
            ctx.remove_texture(*tex_id);
        }

        // Upload pending textures
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
    }, None);

    if let Err(e) = result {
        log::error!("render_frame upload pass failed: {e}");
    }

    // Apply new texture IDs BEFORE building the draw list
    NEW_TEXTURES.with(|c| {
        for (wid, tex_id) in c.borrow().iter() {
            if let Some(w) = state.windows.get_mut(*wid) {
                w.texture_id = Some(*tex_id);
            }
        }
    });

    // Build draw list with up-to-date texture IDs
    let window_draw_list: Vec<((i32, i32), (u32, u32), Option<TextureId>)> =
        state.focus_stack.iter_back_to_front()
            .filter_map(|wid| {
                let w = state.windows.get(wid)?;
                if !w.mapped { return None; }
                Some((w.position, w.size, w.texture_id))
            })
            .collect();

    let result = state.render_state.render_frame(&state.gpu, |ctx| {
        let (sw, sh) = ctx.surface_size();

        // Background
        ctx.draw_rect(Rect {
            position: [sw as f32 / 2.0, sh as f32 / 2.0],
            size: [sw as f32 / 2.0, sh as f32 / 2.0],
            color: Color::rgb(0.15, 0.15, 0.2),
            texture: None,
        });

        // Windows back to front
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
    }, None);

    if let Err(e) = result {
        log::error!("render_frame draw pass failed: {e}");
    }
}

/// Returns the window under the given pointer coordinates (front-to-back order).
pub fn hit_test_window(
    focus_stack: &FocusStack,
    windows: &slotmap::SlotMap<WindowId, crate::shell::window::ManagedWindow>,
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
