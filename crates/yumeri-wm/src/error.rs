use std::io;

#[derive(Debug, thiserror::Error)]
pub enum WmError {
    #[error("Wayland connection error: {0}")]
    WaylandConnect(#[from] wayland_client::ConnectError),

    #[error("Wayland backend error: {0}")]
    WaylandBackend(#[from] wayland_client::backend::WaylandError),

    #[error("Wayland dispatch error: {0}")]
    WaylandDispatch(#[from] wayland_client::DispatchError),

    #[error("Renderer error: {0}")]
    Renderer(#[from] yumeri_renderer::RendererError),

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    General(String),
}

pub type Result<T> = std::result::Result<T, WmError>;
