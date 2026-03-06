use std::os::fd::AsFd;

use wayland_server::protocol::{wl_keyboard, wl_pointer, wl_seat, wl_touch};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::compositor::CompositorState;

impl GlobalDispatch<wl_seat::WlSeat, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<wl_seat::WlSeat>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let seat = data_init.init(resource, ());
        seat.capabilities(wl_seat::Capability::Keyboard | wl_seat::Capability::Pointer);
        seat.name("seat0".into());
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &wl_seat::WlSeat,
        request: wl_seat::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_seat::Request::GetKeyboard { id } => {
                let keyboard = data_init.init(id, ());
                if let Some(ref fd) = state.keyboard_keymap_fd {
                    send_keymap_to_keyboard(&keyboard, fd, state.keyboard_keymap_size);
                }
                state.keyboards.push(keyboard);
            }
            wl_seat::Request::GetPointer { id } => {
                let pointer = data_init.init(id, ());
                state.pointers.push(pointer);
            }
            wl_seat::Request::GetTouch { id } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_keyboard::WlKeyboard,
        _request: wl_keyboard::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {}

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &wl_keyboard::WlKeyboard,
        _data: &(),
    ) {
        state.keyboards.retain(|k| k.id() != resource.id());
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_pointer::WlPointer,
        _request: wl_pointer::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {}

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &wl_pointer::WlPointer,
        _data: &(),
    ) {
        state.pointers.retain(|p| p.id() != resource.id());
    }
}

impl Dispatch<wl_touch::WlTouch, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_touch::WlTouch,
        _request: wl_touch::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {}
}

pub fn create_keymap_fd() -> Option<(std::os::fd::OwnedFd, u32)> {
    let context = xkbcommon::xkb::Context::new(xkbcommon::xkb::CONTEXT_NO_FLAGS);
    let keymap = xkbcommon::xkb::Keymap::new_from_names(
        &context,
        "",
        "",
        "",
        "",
        None,
        xkbcommon::xkb::KEYMAP_COMPILE_NO_FLAGS,
    )?;

    let keymap_str = keymap.get_as_string(xkbcommon::xkb::KEYMAP_FORMAT_TEXT_V1);
    let keymap_bytes = keymap_str.as_bytes();
    let size = keymap_bytes.len() + 1; // null terminator

    let name = std::ffi::CString::new("yumeri-wm-keymap").ok()?;
    let fd = rustix::fs::memfd_create(&name, rustix::fs::MemfdFlags::CLOEXEC).ok()?;

    use std::io::Write;
    let mut file = std::fs::File::from(fd);
    file.write_all(keymap_bytes).ok()?;
    file.write_all(&[0]).ok()?;

    let fd = std::os::fd::OwnedFd::from(file);
    Some((fd, size as u32))
}

pub fn send_keymap_to_keyboard(
    keyboard: &wl_keyboard::WlKeyboard,
    keymap_fd: &std::os::fd::OwnedFd,
    keymap_size: u32,
) {
    keyboard.keymap(
        wl_keyboard::KeymapFormat::XkbV1,
        keymap_fd.as_fd(),
        keymap_size,
    );
}
