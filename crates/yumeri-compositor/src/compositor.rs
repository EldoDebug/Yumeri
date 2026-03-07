use std::collections::HashMap;

use slotmap::SlotMap;
use wayland_protocols::xdg::shell::server::{xdg_surface, xdg_toplevel};
use wayland_server::protocol::{wl_buffer, wl_keyboard, wl_pointer, wl_shm};
use wayland_server::backend::ObjectId;
use wayland_server::Resource;
use yumeri_renderer::{GpuContext, TextureId, WindowRenderState};
use yumeri_shell::LayerShell;
use yumeri_wm::{FocusStack, LayoutEngine, WindowId};

use crate::backend::WaylandBackend;

pub struct ManagedWindow {
    pub surface: wayland_server::protocol::wl_surface::WlSurface,
    pub xdg_toplevel: Option<xdg_toplevel::XdgToplevel>,
    pub xdg_surface: Option<xdg_surface::XdgSurface>,
    pub xdg_surface_id: Option<ObjectId>,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub title: String,
    pub app_id: String,
    pub texture_id: Option<TextureId>,
    pub mapped: bool,
}

pub struct CompositorState {
    pub backend: WaylandBackend,
    pub gpu: GpuContext,
    pub render_state: WindowRenderState,

    pub surfaces: HashMap<ObjectId, SurfaceState>,
    pub shm_pools: HashMap<ObjectId, ShmPoolState>,
    pub shm_buffers: HashMap<ObjectId, ShmBufferSpec>,

    pub windows: SlotMap<WindowId, ManagedWindow>,
    pub focus_stack: FocusStack,
    pub layout_engine: LayoutEngine,
    pub layer_shell: LayerShell,

    pub xdg_surface_window_map: HashMap<ObjectId, WindowId>,
    pub surface_window_map: HashMap<ObjectId, WindowId>,

    pub output_size: (u32, u32),

    pub pointer_x: f64,
    pub pointer_y: f64,
    pub pointer_focus: Option<WindowId>,
    pub keyboard_focus: Option<WindowId>,

    pub keyboard_keymap_fd: Option<std::os::fd::OwnedFd>,
    pub keyboard_keymap_size: u32,

    pub grab: Option<Grab>,
    pub serial_counter: u32,

    pub keyboards: Vec<wl_keyboard::WlKeyboard>,
    pub pointers: Vec<wl_pointer::WlPointer>,

    pub running: bool,
    pub frame_requested: bool,
    pub pending_texture_removals: Vec<TextureId>,
    pub pending_images: Vec<(WindowId, yumeri_image::Image)>,
}

#[derive(Debug, Copy, Clone)]
pub enum Grab {
    Move {
        window_id: WindowId,
        start_pointer: (f64, f64),
        start_position: (i32, i32),
    },
    Resize {
        window_id: WindowId,
        start_pointer: (f64, f64),
        start_size: (u32, u32),
        start_position: (i32, i32),
        edges: u32,
    },
}

pub struct SurfaceState {
    pub buffer: Option<wl_buffer::WlBuffer>,
    pub buffer_spec: Option<ShmBufferSpec>,
    pub committed: bool,
    pub damage: Vec<(i32, i32, i32, i32)>,
    pub frame_callbacks: Vec<wayland_server::protocol::wl_callback::WlCallback>,
}

impl SurfaceState {
    pub fn new() -> Self {
        Self {
            buffer: None,
            buffer_spec: None,
            committed: false,
            damage: Vec::new(),
            frame_callbacks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShmBufferSpec {
    pub pool_id: ObjectId,
    pub offset: i32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub format: wl_shm::Format,
}

pub struct ShmPoolState {
    pub mmap: memmap2::MmapMut,
    pub fd: std::os::fd::OwnedFd,
    pub size: usize,
}

impl CompositorState {
    pub fn next_serial(&mut self) -> u32 {
        self.serial_counter += 1;
        self.serial_counter
    }

    pub fn remove_window(&mut self, wid: WindowId) {
        self.focus_stack.remove(wid);
        let mut xdg_surface_id = None;
        if let Some(w) = self.windows.get(wid) {
            self.surface_window_map.remove(&w.surface.id());
            xdg_surface_id = w.xdg_surface_id.clone();
            if let Some(tex_id) = w.texture_id {
                self.pending_texture_removals.push(tex_id);
            }
        }
        self.windows.remove(wid);
        if let Some(id) = xdg_surface_id {
            self.xdg_surface_window_map.remove(&id);
        }
        if self.keyboard_focus == Some(wid) {
            self.keyboard_focus = None;
        }
        if self.pointer_focus == Some(wid) {
            self.pointer_focus = None;
        }
        if matches!(self.grab, Some(Grab::Move { window_id, .. } | Grab::Resize { window_id, .. }) if window_id == wid) {
            self.grab = None;
        }
    }
}
