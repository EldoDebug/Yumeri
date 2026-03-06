use std::collections::HashMap;

use slotmap::SlotMap;
use wayland_server::protocol::{wl_buffer, wl_keyboard, wl_pointer, wl_shm};
use wayland_server::backend::ObjectId;

use wayland_server::Resource;
use yumeri_renderer::{GpuContext, TextureId, WindowRenderState};

use crate::backend::Backend;
use crate::shell::focus::FocusStack;
use crate::shell::window::{ManagedWindow, WindowId};

pub struct CompositorState {
    pub backend: Box<dyn Backend>,
    pub gpu: GpuContext,
    pub render_state: WindowRenderState,

    pub surfaces: HashMap<ObjectId, SurfaceState>,
    pub shm_pools: HashMap<ObjectId, ShmPoolState>,
    pub shm_buffers: HashMap<ObjectId, ShmBufferSpec>,

    pub windows: SlotMap<WindowId, ManagedWindow>,
    pub focus_stack: FocusStack,

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
    pub next_window_position: (i32, i32),
    pub serial_counter: u32,

    pub keyboards: Vec<wl_keyboard::WlKeyboard>,
    pub pointers: Vec<wl_pointer::WlPointer>,

    pub running: bool,
    pub frame_requested: bool,
    pub pending_texture_removals: Vec<TextureId>,
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
        if let Some(w) = self.windows.get(wid) {
            self.surface_window_map.remove(&w.surface.id());
            if let Some(tex_id) = w.texture_id {
                self.pending_texture_removals.push(tex_id);
            }
        }
        self.windows.remove(wid);
        self.xdg_surface_window_map.retain(|_, &mut v| v != wid);
        if self.keyboard_focus == Some(wid) {
            self.keyboard_focus = None;
        }
        if self.pointer_focus == Some(wid) {
            self.pointer_focus = None;
        }
        // Clear grab if it references the removed window
        if matches!(self.grab, Some(Grab::Move { window_id, .. } | Grab::Resize { window_id, .. }) if window_id == wid) {
            self.grab = None;
        }
    }

    pub fn allocate_window_position(&mut self) -> (i32, i32) {
        let pos = self.next_window_position;
        self.next_window_position.0 += 30;
        self.next_window_position.1 += 30;
        if self.next_window_position.0 > self.output_size.0 as i32 - 200 {
            self.next_window_position.0 = 60;
        }
        if self.next_window_position.1 > self.output_size.1 as i32 - 200 {
            self.next_window_position.1 = 60;
        }
        pos
    }
}
