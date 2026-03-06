use std::time::Duration;

use wayland_server::Display;

use crate::compositor::CompositorState;
use crate::input;
use crate::render;
use crate::shell::window::WindowId;

pub fn run(
    mut display: Display<CompositorState>,
    mut state: CompositorState,
    listener: wayland_server::ListeningSocket,
) -> crate::error::Result<()> {
    log::info!("Entering event loop");

    while state.running {
        // Accept new client connections
        while let Some(stream) = listener.accept().map_err(crate::error::WmError::Io)? {
            if let Err(e) = display.handle().insert_client(
                stream,
                std::sync::Arc::new(()),
            ) {
                log::error!("Failed to insert client: {e}");
            }
        }

        // Dispatch backend events
        state.backend.dispatch()?;
        while let Some(event) = state.backend.next_event() {
            input::handle_backend_event(&mut state, event);
        }

        // Dispatch Wayland server events
        if let Err(e) = display.dispatch_clients(&mut state) {
            log::error!("dispatch_clients error: {e}");
        }
        if let Err(e) = display.flush_clients() {
            log::error!("flush_clients error: {e}");
        }

        // Process committed surfaces -> stage images for GPU upload
        process_surface_commits(&mut state);

        // Render frame (including pending texture uploads)
        if state.frame_requested {
            state.frame_requested = false;
            render::render_frame(&mut state);
            state.backend.present();
            send_frame_callbacks(&mut state);
        } else {
            // No frame to render; sleep briefly to avoid busy-waiting
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    log::info!("Shutting down");
    state.render_state.destroy(&state.gpu);

    Ok(())
}

fn process_surface_commits(state: &mut CompositorState) {
    let surface_ids: Vec<_> = state.surfaces.keys().cloned().collect();
    let mut new_images = Vec::new();

    for surface_id in surface_ids {
        let Some(surf) = state.surfaces.get_mut(&surface_id) else {
            continue;
        };

        if !surf.committed {
            continue;
        }
        surf.committed = false;

        let Some(ref spec) = surf.buffer_spec else {
            continue;
        };
        let pool_id = spec.pool_id.clone();
        let width = spec.width as u32;
        let height = spec.height as u32;

        let image = {
            let Some(pool) = state.shm_pools.get(&pool_id) else {
                continue;
            };
            let Some(ref spec) = state.surfaces.get(&surface_id).and_then(|s| s.buffer_spec.as_ref()) else {
                continue;
            };
            render::surface_texture::shm_buffer_to_image(&pool.mmap, spec)
        };

        let Some(image) = image else { continue };

        // Release the buffer
        if let Some(surf) = state.surfaces.get_mut(&surface_id) {
            if let Some(ref buf) = surf.buffer {
                buf.release();
            }
            surf.damage.clear();
        }

        if let Some(&wid) = state.surface_window_map.get(&surface_id) {
            let mut just_mapped = false;
            if let Some(w) = state.windows.get_mut(wid) {
                w.size = (width, height);
                if !w.mapped {
                    w.mapped = true;
                    just_mapped = true;
                }
            }
            new_images.push((wid, image));

            // Auto-focus newly mapped windows
            if just_mapped {
                crate::input::set_keyboard_focus(state, Some(wid));
            }
        }
    }

    if !new_images.is_empty() {
        PENDING_IMAGES.lock().unwrap().extend(new_images);
    }
}

use std::sync::Mutex;

static PENDING_IMAGES: Mutex<Vec<(WindowId, yumeri_image::Image)>> = Mutex::new(Vec::new());

pub fn take_pending_images() -> Vec<(WindowId, yumeri_image::Image)> {
    std::mem::take(&mut PENDING_IMAGES.lock().unwrap())
}

fn send_frame_callbacks(state: &mut CompositorState) {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u32;

    let surface_ids: Vec<_> = state.surfaces.keys().cloned().collect();
    for surface_id in surface_ids {
        if let Some(surf) = state.surfaces.get_mut(&surface_id) {
            let callbacks: Vec<_> = surf.frame_callbacks.drain(..).collect();
            for cb in callbacks {
                cb.done(time);
            }
        }
    }
}
