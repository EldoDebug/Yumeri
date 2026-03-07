use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Vulkan error: {0}")]
    Vulkan(#[from] ash::vk::Result),

    #[error("allocation error: {0}")]
    Allocation(#[from] gpu_allocator::AllocationError),

    #[error("no suitable GPU found")]
    NoSuitableGpu,

    #[error("no suitable queue family")]
    NoSuitableQueueFamily,

    #[error("swapchain error: {0}")]
    Swapchain(String),

    #[error("allocator initialization error: {0}")]
    AllocatorInit(String),

    #[error("shader error: {0}")]
    Shader(String),

    #[error("texture error: {0}")]
    Texture(String),

    #[error("not initialized: {0}")]
    NotInitialized(String),

    #[error("post-effect error: {0}")]
    PostEffect(String),

    #[error("window handle error: {0:?}")]
    WindowHandle(raw_window_handle::HandleError),
}

impl From<raw_window_handle::HandleError> for RendererError {
    fn from(e: raw_window_handle::HandleError) -> Self {
        Self::WindowHandle(e)
    }
}

pub type Result<T> = std::result::Result<T, RendererError>;
