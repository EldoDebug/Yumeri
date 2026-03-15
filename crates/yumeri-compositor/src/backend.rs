use std::collections::VecDeque;
use std::ptr::NonNull;

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use wayland_client::protocol::{wl_compositor, wl_keyboard, wl_pointer, wl_registry, wl_seat, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::error::{Result, WmError};

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Input(yumeri_input::InputEvent),
    Resize { width: u32, height: u32 },
    FrameRequest,
    Shutdown,
}

pub struct WaylandBackend {
    conn: Connection,
    queue: wayland_client::EventQueue<BackendState>,
    state: BackendState,
}

struct BackendState {
    compositor: Option<wl_compositor::WlCompositor>,
    xdg_wm_base: Option<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase>,
    seat: Option<wl_seat::WlSeat>,
    surface: Option<wl_surface::WlSurface>,
    xdg_surface: Option<wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface>,
    xdg_toplevel: Option<wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel>,
    configured: bool,
    width: u32,
    height: u32,
    events: VecDeque<BackendEvent>,
    closed: bool,
    has_keyboard: bool,
    has_pointer: bool,
    pointer_position: (f64, f64),
}

impl BackendState {
    fn new(width: u32, height: u32) -> Self {
        Self {
            compositor: None,
            xdg_wm_base: None,
            seat: None,
            surface: None,
            xdg_surface: None,
            xdg_toplevel: None,
            configured: false,
            width,
            height,
            events: VecDeque::new(),
            closed: false,
            has_keyboard: false,
            has_pointer: false,
            pointer_position: (0.0, 0.0),
        }
    }
}

impl WaylandBackend {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let conn = Connection::connect_to_env()?;
        let display = conn.display();
        let mut queue = conn.new_event_queue::<BackendState>();
        let qh = queue.handle();

        let mut state = BackendState::new(width, height);
        display.get_registry(&qh, ());
        queue.roundtrip(&mut state).map_err(WmError::WaylandDispatch)?;

        let compositor = state
            .compositor
            .as_ref()
            .ok_or_else(|| WmError::General("no wl_compositor".into()))?;
        let surface = compositor.create_surface(&qh, ());
        state.surface = Some(surface.clone());

        let xdg_wm_base = state
            .xdg_wm_base
            .as_ref()
            .ok_or_else(|| WmError::General("no xdg_wm_base".into()))?;
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, &qh, ());
        state.xdg_surface = Some(xdg_surface);

        let toplevel = state
            .xdg_surface
            .as_ref()
            .unwrap()
            .get_toplevel(&qh, ());
        toplevel.set_title("Yumeri Compositor".into());
        toplevel.set_app_id("yumeri-compositor".into());
        state.xdg_toplevel = Some(toplevel);

        surface.commit();

        while !state.configured {
            queue.blocking_dispatch(&mut state).map_err(WmError::WaylandDispatch)?;
        }

        Ok(Self { conn, queue, state })
    }

    fn wl_display_ptr(&self) -> *mut std::ffi::c_void {
        self.conn.backend().display_ptr() as *mut std::ffi::c_void
    }

    fn wl_surface_ptr(&self) -> *mut std::ffi::c_void {
        use wayland_client::Proxy;
        let surface = self.state.surface.as_ref().unwrap();
        surface.id().as_ptr() as *mut std::ffi::c_void
    }

    pub fn dispatch(&mut self) -> Result<()> {
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(WmError::WaylandDispatch)?;
        if let Some(guard) = self.conn.prepare_read() {
            let _ = guard.read();
        }
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(WmError::WaylandDispatch)?;

        if self.state.closed {
            self.state.events.push_back(BackendEvent::Shutdown);
        }
        Ok(())
    }

    pub fn next_event(&mut self) -> Option<BackendEvent> {
        self.state.events.pop_front()
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        let ptr = NonNull::new(self.wl_display_ptr()).unwrap();
        RawDisplayHandle::Wayland(WaylandDisplayHandle::new(ptr))
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        let ptr = NonNull::new(self.wl_surface_ptr()).unwrap();
        RawWindowHandle::Wayland(WaylandWindowHandle::new(ptr))
    }

    pub fn output_size(&self) -> (u32, u32) {
        (self.state.width, self.state.height)
    }

    pub fn present(&mut self) {
        if let Some(surface) = &self.state.surface {
            surface.frame(&self.queue.handle(), ());
            surface.commit();
        }
    }
}

// --- Dispatch implementations for the client-side backend ---

impl Dispatch<wl_registry::WlRegistry, ()> for BackendState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_compositor" => {
                    state.compositor =
                        Some(registry.bind::<wl_compositor::WlCompositor, _, _>(name, version.min(6), qh, ()));
                }
                "xdg_wm_base" => {
                    state.xdg_wm_base =
                        Some(registry.bind::<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase, _, _>(name, version.min(3), qh, ()));
                }
                "wl_seat" => {
                    let seat =
                        registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(8), qh, ());
                    state.seat = Some(seat);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for BackendState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_compositor::WlCompositor,
        _event: wl_compositor::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<wl_surface::WlSurface, ()> for BackendState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_surface::WlSurface,
        _event: wl_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {}
}

impl Dispatch<wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase, ()> for BackendState {
    fn event(
        _state: &mut Self,
        wm_base: &wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase,
        event: wayland_protocols::xdg::shell::client::xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface, ()> for BackendState {
    fn event(
        state: &mut Self,
        xdg_surface: &wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface,
        event: wayland_protocols::xdg::shell::client::xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wayland_protocols::xdg::shell::client::xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            if let Some(surface) = &state.surface {
                surface.commit();
            }
            state.configured = true;
        }
    }
}

impl Dispatch<wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel, ()> for BackendState {
    fn event(
        state: &mut Self,
        _toplevel: &wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel,
        event: wayland_protocols::xdg::shell::client::xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wayland_protocols::xdg::shell::client::xdg_toplevel::Event::Configure {
                width,
                height,
                states: _,
            } => {
                if width > 0 && height > 0 {
                    let new_w = width as u32;
                    let new_h = height as u32;
                    if new_w != state.width || new_h != state.height {
                        state.width = new_w;
                        state.height = new_h;
                        state.events.push_back(BackendEvent::Resize {
                            width: new_w,
                            height: new_h,
                        });
                    }
                }
            }
            wayland_protocols::xdg::shell::client::xdg_toplevel::Event::Close => {
                state.closed = true;
            }
            _ => {}
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for BackendState {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: cap,
        } = event
        {
            let cap = wl_seat::Capability::from_bits_truncate(cap.into());
            if cap.contains(wl_seat::Capability::Keyboard) && !state.has_keyboard {
                seat.get_keyboard(qh, ());
                state.has_keyboard = true;
            }
            if cap.contains(wl_seat::Capability::Pointer) && !state.has_pointer {
                seat.get_pointer(qh, ());
                state.has_pointer = true;
            }
        }
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for BackendState {
    fn event(
        this: &mut Self,
        _keyboard: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            serial: _,
            time: _,
            key,
            state,
        } = event
        {
            let pressed = matches!(state, wayland_client::WEnum::Value(wl_keyboard::KeyState::Pressed));
            let btn_state = if pressed {
                yumeri_input::ButtonState::Pressed
            } else {
                yumeri_input::ButtonState::Released
            };
            this.events.push_back(BackendEvent::Input(
                yumeri_input::InputEvent::Keyboard(yumeri_input::KeyboardEvent {
                    key: yumeri_input::Key::Unidentified,
                    code: yumeri_input::KeyCode::Other(key),
                    state: btn_state,
                    modifiers: yumeri_input::Modifiers::NONE,
                    text: None,
                    repeat: false,
                }),
            ));
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for BackendState {
    fn event(
        this: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_pointer::Event::Motion {
                time: _,
                surface_x,
                surface_y,
            } => {
                this.pointer_position = (surface_x, surface_y);
                this.events.push_back(BackendEvent::Input(
                    yumeri_input::InputEvent::Pointer(yumeri_input::PointerEvent {
                        kind: yumeri_input::PointerEventKind::Moved,
                        position: this.pointer_position,
                        modifiers: yumeri_input::Modifiers::NONE,
                    }),
                ));
            }
            wl_pointer::Event::Button {
                serial: _,
                time: _,
                button,
                state,
            } => {
                let pressed = matches!(state, wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed));
                let mb = yumeri_input::MouseButton::from_linux_evdev(button);
                let kind = if pressed {
                    yumeri_input::PointerEventKind::ButtonPressed(mb)
                } else {
                    yumeri_input::PointerEventKind::ButtonReleased(mb)
                };
                this.events.push_back(BackendEvent::Input(
                    yumeri_input::InputEvent::Pointer(yumeri_input::PointerEvent {
                        kind,
                        position: this.pointer_position,
                        modifiers: yumeri_input::Modifiers::NONE,
                    }),
                ));
            }
            _ => {}
        }
    }
}

impl Dispatch<wayland_client::protocol::wl_callback::WlCallback, ()> for BackendState {
    fn event(
        state: &mut Self,
        _cb: &wayland_client::protocol::wl_callback::WlCallback,
        event: wayland_client::protocol::wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_callback::Event::Done { .. } = event {
            state.events.push_back(BackendEvent::FrameRequest);
        }
    }
}
