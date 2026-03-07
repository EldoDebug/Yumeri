pub mod compositor;
pub mod output;
pub mod seat;
pub mod shm;
pub mod xdg_shell;

use wayland_server::Display;

use crate::compositor::CompositorState;

pub fn register_globals(display: &mut Display<CompositorState>) {
    let dh = display.handle();
    dh.create_global::<CompositorState, wayland_server::protocol::wl_compositor::WlCompositor, _>(
        6, (),
    );
    dh.create_global::<CompositorState, wayland_server::protocol::wl_shm::WlShm, _>(1, ());
    dh.create_global::<CompositorState, wayland_server::protocol::wl_output::WlOutput, _>(4, ());
    dh.create_global::<CompositorState, wayland_server::protocol::wl_seat::WlSeat, _>(8, ());
    dh.create_global::<CompositorState, wayland_protocols::xdg::shell::server::xdg_wm_base::XdgWmBase, _>(
        3, (),
    );
}
