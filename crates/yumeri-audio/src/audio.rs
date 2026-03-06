use std::path::Path;
use std::time::Duration;

use crate::decode;
use crate::error::Result;
use crate::sample_format::SampleFormat;

#[derive(Clone, Debug)]
pub struct Audio {
    data: Vec<u8>,
    sample_rate: u32,
    channels: u16,
    format: SampleFormat,
}

impl Audio {
    pub fn from_raw(data: Vec<u8>, sample_rate: u32, channels: u16, format: SampleFormat) -> Self {
        let frame_size = format.bytes_per_sample() * channels as usize;
        debug_assert!(
            frame_size == 0 || data.len() % frame_size == 0,
            "buffer size {} is not a multiple of frame size ({frame_size})",
            data.len(),
        );
        Self {
            data,
            sample_rate,
            channels,
            format,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_with(path, SampleFormat::default())
    }

    pub fn load_with(path: impl AsRef<Path>, format: SampleFormat) -> Result<Self> {
        decode::decode_from_path(path.as_ref(), format)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        Self::decode_with(bytes, SampleFormat::default())
    }

    pub fn decode_with(bytes: &[u8], format: SampleFormat) -> Result<Self> {
        decode::decode_from_memory(bytes, format)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn format(&self) -> SampleFormat {
        self.format
    }

    pub fn frame_count(&self) -> usize {
        let frame_size = self.format.bytes_per_sample() * self.channels as usize;
        if frame_size == 0 {
            return 0;
        }
        self.data.len() / frame_size
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(self.duration_secs())
    }

    pub fn duration_secs(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frame_count() as f64 / self.sample_rate as f64
    }

    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    pub fn convert_to(&self, target: SampleFormat) -> Result<Self> {
        decode::convert(self, target)
    }
}
