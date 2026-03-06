use crate::pixel_format::VideoPixelFormat;

/// A decoded video frame, either on CPU or GPU.
pub enum VideoFrame {
    Cpu {
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: VideoPixelFormat,
        pts: f64,
    },
    Gpu(GpuFrame),
}

/// A video frame residing on the GPU as Vulkan images (zero-copy from FFmpeg Vulkan hwaccel).
pub struct GpuFrame {
    pub(crate) images: [ash::vk::Image; 2],
    pub(crate) layouts: [ash::vk::ImageLayout; 2],
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pts: f64,
    pub(crate) _frame_ref: std::sync::Arc<FrameRef>,
}

/// Holds a reference to the underlying AVFrame to keep FFmpeg's reference count alive.
pub(crate) struct FrameRef {
    pub(crate) _frame: rsmpeg::avutil::AVFrame,
}

// Safety: AVFrame data is on GPU, we only hold the reference for lifetime management.
unsafe impl Send for FrameRef {}
unsafe impl Sync for FrameRef {}

impl GpuFrame {
    pub fn images(&self) -> [ash::vk::Image; 2] {
        self.images
    }

    pub fn layouts(&self) -> [ash::vk::ImageLayout; 2] {
        self.layouts
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn pts(&self) -> f64 {
        self.pts
    }
}

impl VideoFrame {
    pub fn pts(&self) -> f64 {
        match self {
            Self::Cpu { pts, .. } => *pts,
            Self::Gpu(gpu) => gpu.pts,
        }
    }

    pub fn width(&self) -> u32 {
        match self {
            Self::Cpu { width, .. } => *width,
            Self::Gpu(gpu) => gpu.width,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Cpu { height, .. } => *height,
            Self::Gpu(gpu) => gpu.height,
        }
    }
}
