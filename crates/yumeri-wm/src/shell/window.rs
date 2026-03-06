use wayland_server::protocol::wl_surface::WlSurface;
use yumeri_renderer::TextureId;

slotmap::new_key_type! { pub struct WindowId; }

pub struct ManagedWindow {
    pub surface: WlSurface,
    pub position: (i32, i32),
    pub size: (u32, u32),
    pub title: String,
    pub app_id: String,
    pub texture_id: Option<TextureId>,
    pub mapped: bool,
}
