use wayland_server::protocol::{
    wl_compositor, wl_region, wl_surface, wl_callback, wl_buffer,
};
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::compositor::{CompositorState, SurfaceState};

// --- wl_compositor ---

impl GlobalDispatch<wl_compositor::WlCompositor, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<wl_compositor::WlCompositor>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        _resource: &wl_compositor::WlCompositor,
        request: wl_compositor::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        match request {
            wl_compositor::Request::CreateSurface { id } => {
                let surface = data_init.init(id, ());
                let obj_id = surface.id();
                state.surfaces.insert(obj_id, SurfaceState::new());
            }
            wl_compositor::Request::CreateRegion { id } => {
                data_init.init(id, ());
            }
            _ => {}
        }
    }
}

// --- wl_surface ---

impl Dispatch<wl_surface::WlSurface, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        surface: &wl_surface::WlSurface,
        request: wl_surface::Request,
        _data: &(),
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, Self>,
    ) {
        let surface_id = surface.id();
        match request {
            wl_surface::Request::Attach { buffer, x: _, y: _ } => {
                if let Some(surf_state) = state.surfaces.get_mut(&surface_id) {
                    if let Some(ref buf) = buffer {
                        let buf_id = buf.id();
                        surf_state.buffer = buffer;
                        surf_state.buffer_spec = state.shm_buffers.get(&buf_id).cloned();
                    } else {
                        surf_state.buffer = None;
                        surf_state.buffer_spec = None;
                    }
                }
            }
            wl_surface::Request::Damage { x, y, width, height }
            | wl_surface::Request::DamageBuffer { x, y, width, height } => {
                if let Some(surf_state) = state.surfaces.get_mut(&surface_id) {
                    surf_state.damage.push((x, y, width, height));
                }
            }
            wl_surface::Request::Frame { callback } => {
                let cb = data_init.init(callback, ());
                if let Some(surf_state) = state.surfaces.get_mut(&surface_id) {
                    surf_state.frame_callbacks.push(cb);
                }
            }
            wl_surface::Request::Commit => {
                if let Some(surf_state) = state.surfaces.get_mut(&surface_id) {
                    surf_state.committed = true;
                }
                state.frame_requested = true;
            }
            wl_surface::Request::Destroy => {
                state.surfaces.remove(&surface_id);
            }
            wl_surface::Request::SetOpaqueRegion { .. }
            | wl_surface::Request::SetInputRegion { .. }
            | wl_surface::Request::SetBufferTransform { .. }
            | wl_surface::Request::SetBufferScale { .. }
            | wl_surface::Request::Offset { .. } => {}
            _ => {}
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &wl_surface::WlSurface,
        _data: &(),
    ) {
        let surface_id = resource.id();
        state.surfaces.remove(&surface_id);

        if let Some(&wid) = state.surface_window_map.get(&surface_id) {
            state.remove_window(wid);
        }
    }
}

// --- wl_region ---

impl Dispatch<wl_region::WlRegion, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_region::WlRegion,
        _request: wl_region::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // Regions are not used in this simple WM
    }
}

// --- wl_callback (frame callbacks) ---
// wl_callback has no client requests; Dispatch is required by wayland-server but never called.

impl Dispatch<wl_callback::WlCallback, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_callback::WlCallback,
        _request: wl_callback::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {}
}

// --- wl_buffer ---

impl Dispatch<wl_buffer::WlBuffer, ()> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &Client,
        resource: &wl_buffer::WlBuffer,
        request: wl_buffer::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        if let wl_buffer::Request::Destroy = request {
            state.shm_buffers.remove(&resource.id());
        }
    }

    fn destroyed(
        state: &mut Self,
        _client: wayland_server::backend::ClientId,
        resource: &wl_buffer::WlBuffer,
        _data: &(),
    ) {
        state.shm_buffers.remove(&resource.id());
    }
}
