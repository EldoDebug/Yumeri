use wayland_server::protocol::wl_output;
use wayland_server::{Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource};

use crate::compositor::CompositorState;

impl GlobalDispatch<wl_output::WlOutput, ()> for CompositorState {
    fn bind(
        state: &mut Self,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<wl_output::WlOutput>,
        _global_data: &(),
        data_init: &mut DataInit<'_, Self>,
    ) {
        let output = data_init.init(resource, ());
        let (w, h) = state.output_size;

        // Physical size in mm, assuming ~96 DPI
        let phys_w = (w as i32) * 254 / 960;
        let phys_h = (h as i32) * 254 / 960;
        output.geometry(
            0,
            0,
            phys_w,
            phys_h,
            wl_output::Subpixel::None,
            "Yumeri".into(),
            "Virtual Display".into(),
            wl_output::Transform::Normal,
        );
        output.mode(
            wl_output::Mode::Current | wl_output::Mode::Preferred,
            w as i32,
            h as i32,
            60000,
        );
        output.scale(1);

        if output.version() >= 2 {
            output.done();
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &Client,
        _resource: &wl_output::WlOutput,
        _request: wl_output::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, Self>,
    ) {
        // wl_output has no client requests besides Release (v3+)
    }
}
