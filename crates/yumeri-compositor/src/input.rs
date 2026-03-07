use wayland_server::protocol::{wl_keyboard, wl_pointer};
use wayland_server::Resource;

use crate::backend::BackendEvent;
use crate::compositor::{CompositorState, Grab};
use crate::render;
use yumeri_wm::WindowId;

const EDGE_TOP: u32 = 1;
const EDGE_BOTTOM: u32 = 2;
const EDGE_LEFT: u32 = 4;
const EDGE_RIGHT: u32 = 8;

const MIN_WINDOW_SIZE: i32 = 100;

const WL_POINTER_FRAME_SINCE: u32 = 5;

pub fn handle_backend_event(state: &mut CompositorState, event: BackendEvent) {
    match event {
        BackendEvent::PointerMotion { x, y, time } => handle_pointer_motion(state, x, y, time),
        BackendEvent::PointerButton {
            button,
            pressed,
            time,
        } => handle_pointer_button(state, button, pressed, time),
        BackendEvent::KeyInput {
            keycode,
            pressed,
            time,
        } => handle_key_input(state, keycode, pressed, time),
        BackendEvent::Resize { width, height } => {
            state.output_size = (width, height);
            state.layout_engine.set_output_size(width, height);
            let _ = state.render_state.on_resize(&state.gpu, width, height);
        }
        BackendEvent::FrameRequest => {
            state.frame_requested = true;
        }
        BackendEvent::Shutdown => {
            state.running = false;
        }
    }
}

fn handle_pointer_motion(state: &mut CompositorState, x: f64, y: f64, time: u32) {
    state.pointer_x = x;
    state.pointer_y = y;

    if let Some(ref grab) = state.grab {
        match *grab {
            Grab::Move {
                window_id,
                start_pointer,
                start_position,
            } => {
                if let Some(w) = state.windows.get_mut(window_id) {
                    w.position.0 = start_position.0 + (x - start_pointer.0) as i32;
                    w.position.1 = start_position.1 + (y - start_pointer.1) as i32;
                }
                return;
            }
            Grab::Resize {
                window_id,
                start_pointer,
                start_size,
                start_position,
                edges,
            } => {
                if let Some(w) = state.windows.get_mut(window_id) {
                    let dx = (x - start_pointer.0) as i32;
                    let dy = (y - start_pointer.1) as i32;
                    let mut new_w = start_size.0 as i32;
                    let mut new_h = start_size.1 as i32;
                    let mut new_x = start_position.0;
                    let mut new_y = start_position.1;

                    if edges & EDGE_RIGHT != 0 { new_w += dx; }
                    if edges & EDGE_LEFT != 0 { new_w -= dx; new_x += dx; }
                    if edges & EDGE_BOTTOM != 0 { new_h += dy; }
                    if edges & EDGE_TOP != 0 { new_h -= dy; new_y += dy; }

                    let clamped_w = new_w.max(MIN_WINDOW_SIZE);
                    let clamped_h = new_h.max(MIN_WINDOW_SIZE);
                    if edges & EDGE_LEFT != 0 { new_x += new_w - clamped_w; }
                    if edges & EDGE_TOP != 0 { new_y += new_h - clamped_h; }

                    w.size = (clamped_w as u32, clamped_h as u32);
                    w.position = (new_x, new_y);
                }
                return;
            }
        }
    }

    let new_focus = render::hit_test_window(&state.focus_stack, &state.windows, x, y);

    if new_focus != state.pointer_focus {
        if let Some(old_wid) = state.pointer_focus {
            send_pointer_leave(state, old_wid);
        }
        if let Some(new_wid) = new_focus {
            send_pointer_enter(state, new_wid, x, y);
        }
        state.pointer_focus = new_focus;
    } else if let Some(wid) = state.pointer_focus {
        send_pointer_motion(state, wid, x, y, time);
    }
}

fn handle_pointer_button(state: &mut CompositorState, button: u32, pressed: bool, time: u32) {
    if !pressed {
        if state.grab.is_some() {
            state.grab = None;
            return;
        }
    }

    if pressed {
        if let Some(wid) = state.pointer_focus {
            state.focus_stack.raise(wid);
            set_keyboard_focus(state, Some(wid));
        }
    }

    if let Some(wid) = state.pointer_focus {
        send_pointer_button(state, wid, button, pressed, time);
    }
}

fn handle_key_input(state: &mut CompositorState, keycode: u32, pressed: bool, time: u32) {
    if let Some(wid) = state.keyboard_focus {
        send_key_event(state, wid, keycode, pressed, time);
    }
}

pub fn set_keyboard_focus(state: &mut CompositorState, new_focus: Option<WindowId>) {
    if state.keyboard_focus == new_focus {
        return;
    }

    let serial = state.next_serial();

    if let Some(old_wid) = state.keyboard_focus {
        if let Some(w) = state.windows.get(old_wid) {
            let target_client = w.surface.client();
            for kb in &state.keyboards {
                if kb.client().as_ref() == target_client.as_ref() {
                    kb.leave(serial, &w.surface);
                }
            }
        }
    }

    if let Some(new_wid) = new_focus {
        if let Some(w) = state.windows.get(new_wid) {
            let target_client = w.surface.client();
            for kb in &state.keyboards {
                if kb.client().as_ref() == target_client.as_ref() {
                    kb.enter(serial, &w.surface, vec![]);
                }
            }
        }
    }

    state.keyboard_focus = new_focus;
}

fn send_pointer_enter(state: &mut CompositorState, wid: WindowId, x: f64, y: f64) {
    let serial = state.next_serial();
    let Some(w) = state.windows.get(wid) else { return };
    let sx = x - w.position.0 as f64;
    let sy = y - w.position.1 as f64;
    let target_client = w.surface.client();

    for ptr in &state.pointers {
        if ptr.client().as_ref() == target_client.as_ref() {
            ptr.enter(serial, &w.surface, sx, sy);
            if ptr.version() >= WL_POINTER_FRAME_SINCE {
                ptr.frame();
            }
        }
    }
}

fn send_pointer_leave(state: &mut CompositorState, wid: WindowId) {
    let serial = state.next_serial();
    let Some(w) = state.windows.get(wid) else { return };
    let target_client = w.surface.client();

    for ptr in &state.pointers {
        if ptr.client().as_ref() == target_client.as_ref() {
            ptr.leave(serial, &w.surface);
            if ptr.version() >= WL_POINTER_FRAME_SINCE {
                ptr.frame();
            }
        }
    }
}

fn send_pointer_motion(state: &mut CompositorState, wid: WindowId, x: f64, y: f64, time: u32) {
    let Some(w) = state.windows.get(wid) else { return };
    let sx = x - w.position.0 as f64;
    let sy = y - w.position.1 as f64;
    let target_client = w.surface.client();

    for ptr in &state.pointers {
        if ptr.client().as_ref() == target_client.as_ref() {
            ptr.motion(time, sx, sy);
            if ptr.version() >= WL_POINTER_FRAME_SINCE {
                ptr.frame();
            }
        }
    }
}

fn send_pointer_button(state: &mut CompositorState, wid: WindowId, button: u32, pressed: bool, time: u32) {
    let serial = state.next_serial();
    let Some(w) = state.windows.get(wid) else { return };
    let btn_state = if pressed {
        wl_pointer::ButtonState::Pressed
    } else {
        wl_pointer::ButtonState::Released
    };
    let target_client = w.surface.client();

    for ptr in &state.pointers {
        if ptr.client().as_ref() == target_client.as_ref() {
            ptr.button(serial, time, button, btn_state);
            if ptr.version() >= WL_POINTER_FRAME_SINCE {
                ptr.frame();
            }
        }
    }
}

fn send_key_event(state: &mut CompositorState, wid: WindowId, keycode: u32, pressed: bool, time: u32) {
    let serial = state.next_serial();
    let Some(w) = state.windows.get(wid) else { return };
    let key_state = if pressed {
        wl_keyboard::KeyState::Pressed
    } else {
        wl_keyboard::KeyState::Released
    };
    let target_client = w.surface.client();

    for kb in &state.keyboards {
        if kb.client().as_ref() == target_client.as_ref() {
            kb.key(serial, time, keycode, key_state);
        }
    }
}
