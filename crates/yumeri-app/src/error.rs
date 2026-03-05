use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("event loop error: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("window creation error: {0}")]
    WindowCreation(#[from] winit::error::OsError),
}
