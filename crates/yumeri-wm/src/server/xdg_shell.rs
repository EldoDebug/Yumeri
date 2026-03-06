use wayland_protocols::xdg::shell::server::{xdg_popup, xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_server::protocol::wl_surface::WlSurface;
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::compositor::{CompositorState, Grab};
use crate::shell::window::ManagedWindow;

const DEFAULT_WINDOW_WIDTH: u32 = 640;
const DEFAULT_WINDOW_HEIGHT: u32 = 480;

// --- xdg_wm_base ---

impl GlobalDispatch<xdg_wm_base::XdgWmBase, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<xdg_wm_base::XdgWmBase>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &xdg_wm_base::XdgWmBase,
        request: xdg_wm_base::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                let surface_id = surface.id();
                data_init.init(id, XdgSurfaceData {
                    surface_id,
                    wl_surface: surface,
                });
            }
            xdg_wm_base::Request::Pong { .. } => {}
            xdg_wm_base::Request::Destroy => {}
            _ => {}
        }
    }
}

// --- xdg_surface ---

pub struct XdgSurfaceData {
    pub surface_id: wayland_server::backend::ObjectId,
    pub wl_surface: WlSurface,
}

impl Dispatch<xdg_surface::XdgSurface, XdgSurfaceData> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &xdg_surface::XdgSurface,
        request: xdg_surface::Request,
        data: &XdgSurfaceData,
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_surface::Request::GetToplevel { id } => {
                let xdg_surface_id = resource.id();

                let toplevel = data_init.init(
                    id,
                    XdgToplevelData {
                        xdg_surface_id: xdg_surface_id.clone(),
                        surface_id: data.surface_id.clone(),
                    },
                );

                // Create the managed window
                let position = state.allocate_window_position();
                let window = ManagedWindow {
                    surface: data.wl_surface.clone(),
                    position,
                    size: (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
                    title: String::new(),
                    app_id: String::new(),
                    texture_id: None,
                    mapped: false,
                };
                let wid = state.windows.insert(window);
                state
                    .xdg_surface_window_map
                    .insert(xdg_surface_id, wid);
                state
                    .surface_window_map
                    .insert(data.surface_id.clone(), wid);
                state.focus_stack.raise(wid);

                // Send initial configure with suggested size and activated state
                let states =
                    (xdg_toplevel::State::Activated as u32).to_ne_bytes().to_vec();
                toplevel.configure(DEFAULT_WINDOW_WIDTH as i32, DEFAULT_WINDOW_HEIGHT as i32, states);
                resource.configure(state.next_serial());
            }
            xdg_surface::Request::GetPopup { id, .. } => {
                data_init.init(id, ());
            }
            xdg_surface::Request::AckConfigure { .. } => {}
            xdg_surface::Request::SetWindowGeometry { .. } => {}
            xdg_surface::Request::Destroy => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&resource.id()) {
                    state.remove_window(wid);
                }
            }
            _ => {}
        }
    }
}

// --- xdg_toplevel ---

pub struct XdgToplevelData {
    pub xdg_surface_id: wayland_server::backend::ObjectId,
    pub surface_id: wayland_server::backend::ObjectId,
}

impl Dispatch<xdg_toplevel::XdgToplevel, XdgToplevelData> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &xdg_toplevel::XdgToplevel,
        request: xdg_toplevel::Request,
        data: &XdgToplevelData,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            xdg_toplevel::Request::SetTitle { title } => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    if let Some(w) = state.windows.get_mut(wid) {
                        w.title = title;
                    }
                }
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    if let Some(w) = state.windows.get_mut(wid) {
                        w.app_id = app_id;
                    }
                }
            }
            xdg_toplevel::Request::Move { .. } => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    if let Some(w) = state.windows.get(wid) {
                        state.grab = Some(Grab::Move {
                            window_id: wid,
                            start_pointer: (state.pointer_x, state.pointer_y),
                            start_position: w.position,
                        });
                    }
                }
            }
            xdg_toplevel::Request::Resize { edges, .. } => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    if let Some(w) = state.windows.get(wid) {
                        let edge_val = match edges {
                            wayland_server::WEnum::Value(e) => e as u32,
                            _ => 0,
                        };
                        state.grab = Some(Grab::Resize {
                            window_id: wid,
                            start_pointer: (state.pointer_x, state.pointer_y),
                            start_size: w.size,
                            start_position: w.position,
                            edges: edge_val,
                        });
                    }
                }
            }
            xdg_toplevel::Request::SetMaximized => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    let (ow, oh) = state.output_size;
                    if let Some(w) = state.windows.get_mut(wid) {
                        w.position = (0, 0);
                        w.size = (ow, oh);
                    }
                    let states =
                        (xdg_toplevel::State::Maximized as u32).to_ne_bytes().to_vec();
                    resource.configure(ow as i32, oh as i32, states);
                }
            }
            xdg_toplevel::Request::UnsetMaximized => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    let pos = state.allocate_window_position();
                    if let Some(w) = state.windows.get_mut(wid) {
                        w.size = (DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT);
                        w.position = pos;
                    }
                    resource.configure(DEFAULT_WINDOW_WIDTH as i32, DEFAULT_WINDOW_HEIGHT as i32, vec![]);
                }
            }
            xdg_toplevel::Request::SetMinimized => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    if let Some(w) = state.windows.get_mut(wid) {
                        w.mapped = false;
                    }
                    state.focus_stack.remove(wid);
                }
            }
            xdg_toplevel::Request::Destroy => {
                if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
                    state.remove_window(wid);
                }
            }
            _ => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        _resource: &xdg_toplevel::XdgToplevel,
        data: &XdgToplevelData,
    ) {
        if let Some(&wid) = state.xdg_surface_window_map.get(&data.xdg_surface_id) {
            state.remove_window(wid);
        }
    }
}

// --- xdg_popup (stub) ---

impl Dispatch<xdg_popup::XdgPopup, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &xdg_popup::XdgPopup,
        _request: xdg_popup::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {}
}
