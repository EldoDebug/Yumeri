#![allow(dead_code)]

mod backend;
mod compositor;
mod error;
mod event_loop;
mod input;
mod render;
mod server;
mod shell;

use std::collections::HashMap;

use slotmap::SlotMap;
use wayland_server::{Display, ListeningSocket};

use backend::WaylandBackend;
use compositor::CompositorState;
use shell::focus::FocusStack;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("Starting Yumeri WM");

    let width = 1280u32;
    let height = 720u32;

    // Initialize backend
    let backend = WaylandBackend::new(width, height)?;

    let display_handle = backend.raw_display_handle();
    let window_handle = backend.raw_window_handle();

    // Initialize GPU
    let gpu = yumeri_renderer::GpuContext::new(display_handle, window_handle)?;
    let render_state =
        yumeri_renderer::WindowRenderState::new(&gpu, display_handle, window_handle, width, height, true, false)?;

    // Initialize Wayland server
    let mut display: Display<CompositorState> = Display::new()?;

    // Register globals
    server::register_globals(&mut display);

    // Set up listening socket
    let listener = ListeningSocket::bind_auto("wayland", 0..33)?;
    let socket_name = listener
        .socket_name()
        .and_then(|n| n.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "wayland-yumeri".into());
    log::info!("Wayland socket: {socket_name}");
    // SAFETY: single-threaded at this point, before any client connections
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &socket_name); }

    // Create keymap
    let (keymap_fd, keymap_size) = server::seat::create_keymap_fd()
        .ok_or_else(|| error::WmError::General("Failed to create keymap".into()))?;

    let state = CompositorState {
        backend,
        gpu,
        render_state,

        surfaces: HashMap::new(),
        shm_pools: HashMap::new(),
        shm_buffers: HashMap::new(),

        windows: SlotMap::with_key(),
        focus_stack: FocusStack::new(),

        xdg_surface_window_map: HashMap::new(),
        surface_window_map: HashMap::new(),

        output_size: (width, height),

        pointer_x: 0.0,
        pointer_y: 0.0,
        pointer_focus: None,
        keyboard_focus: None,

        keyboard_keymap_fd: Some(keymap_fd),
        keyboard_keymap_size: keymap_size,

        grab: None,
        next_window_position: (60, 60),
        serial_counter: 0,

        keyboards: Vec::new(),
        pointers: Vec::new(),

        running: true,
        frame_requested: true,
        pending_texture_removals: Vec::new(),
    };

    event_loop::run(display, state, listener)?;

    Ok(())
}
