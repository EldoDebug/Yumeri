use std::sync::atomic::{AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::{mpsc, Arc};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::audio::Audio;
use crate::decode;
use crate::error::{AudioError, Result};
use crate::sample_format::SampleFormat;

/// Audio output device wrapper. Not thread-safe — use on the thread that created it.
pub struct AudioPlayer {
    config: cpal::StreamConfig,
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;
        let supported = device
            .default_output_config()
            .map_err(|e| AudioError::Playback(e.to_string()))?;

        Ok(Self {
            config: supported.into(),
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    pub fn play(&self, audio: &Audio) -> Result<AudioHandle> {
        self.play_with(audio, 1.0, false)
    }

    pub fn play_with(&self, audio: &Audio, volume: f32, looping: bool) -> Result<AudioHandle> {
        let (samples, src_channels, src_rate) = if audio.format() == SampleFormat::F32 {
            (
                decode::bytes_to_f32_samples(audio.data(), SampleFormat::F32),
                audio.channels() as usize,
                audio.sample_rate(),
            )
        } else {
            let converted = audio.convert_to(SampleFormat::F32)?;
            (
                decode::bytes_to_f32_samples(converted.data(), SampleFormat::F32),
                converted.channels() as usize,
                converted.sample_rate(),
            )
        };
        let samples = Arc::new(samples);

        let dst_channels = self.config.channels as usize;
        let total_src_frames = (samples.len() / src_channels) as u64;
        let rate_ratio = src_rate as f64 / self.config.sample_rate.0 as f64;

        let state = Arc::new(SharedState {
            cursor: AtomicU64::new(0),
            volume: AtomicU32::new(volume.to_bits()),
            playback: AtomicU8::new(PlaybackState::Playing as u8),
            looping: AtomicU8::new(u8::from(looping)),
            total_src_frames,
            dst_rate: self.config.sample_rate.0,
        });

        let cb_samples = Arc::clone(&samples);
        let cb_state = Arc::clone(&state);
        let config = self.config.clone();
        let (result_tx, result_rx) = mpsc::channel::<std::result::Result<(), String>>();
        let (drop_tx, drop_rx) = mpsc::channel::<()>();

        // Spawn a dedicated thread that owns the cpal Device and Stream.
        // This avoids unsafe Send/Sync on platform-specific audio types.
        std::thread::spawn(move || {
            let host = cpal::default_host();
            let device = match host.default_output_device() {
                Some(d) => d,
                None => {
                    let _ = result_tx.send(Err("no output device".into()));
                    return;
                }
            };

            let stream = match device.build_output_stream(
                &config,
                move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let ps = cb_state.playback.load(Ordering::Relaxed);
                    if ps != PlaybackState::Playing as u8 {
                        output.fill(0.0);
                        return;
                    }

                    let vol = f32::from_bits(cb_state.volume.load(Ordering::Relaxed));
                    let is_looping = cb_state.looping.load(Ordering::Relaxed) != 0;
                    let total = cb_state.total_src_frames;
                    let mut cursor = cb_state.cursor.load(Ordering::Relaxed);

                    for frame in output.chunks_mut(dst_channels) {
                        let src_pos = cursor as f64 * rate_ratio;
                        let src_frame = src_pos as u64;

                        if src_frame >= total {
                            if is_looping {
                                cursor = 0;
                                let idx_b = 1.min(total.saturating_sub(1) as usize);
                                fill_frame_lerp(
                                    frame,
                                    &cb_samples,
                                    0,
                                    idx_b,
                                    0.0,
                                    src_channels,
                                    vol,
                                );
                            } else {
                                frame.fill(0.0);
                                cb_state.playback.store(
                                    PlaybackState::Stopped as u8,
                                    Ordering::Relaxed,
                                );
                                cb_state.cursor.store(cursor, Ordering::Relaxed);
                                return;
                            }
                        } else {
                            let idx_a = src_frame as usize;
                            let idx_b = (idx_a + 1).min(total.saturating_sub(1) as usize);
                            let frac = (src_pos - src_pos.floor()) as f32;
                            fill_frame_lerp(
                                frame,
                                &cb_samples,
                                idx_a,
                                idx_b,
                                frac,
                                src_channels,
                                vol,
                            );
                        }

                        cursor += 1;
                    }

                    cb_state.cursor.store(cursor, Ordering::Relaxed);
                },
                |err| log::error!("audio stream error: {err}"),
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    let _ = result_tx.send(Err(e.to_string()));
                    return;
                }
            };

            if let Err(e) = stream.play() {
                let _ = result_tx.send(Err(e.to_string()));
                return;
            }

            let _ = result_tx.send(Ok(()));

            // Keep the stream alive until all AudioHandle clones are dropped.
            // When the last Arc<Sender> drops, this recv returns Err and the thread exits.
            let _ = drop_rx.recv();
        });

        match result_rx.recv() {
            Ok(Ok(())) => Ok(AudioHandle {
                state,
                _drop_guard: Arc::new(drop_tx),
            }),
            Ok(Err(e)) => Err(AudioError::Playback(e)),
            Err(_) => Err(AudioError::Playback("audio thread exited unexpectedly".into())),
        }
    }
}

fn fill_frame_lerp(
    dst: &mut [f32],
    samples: &[f32],
    frame_a: usize,
    frame_b: usize,
    frac: f32,
    src_channels: usize,
    volume: f32,
) {
    let off_a = frame_a * src_channels;
    let off_b = frame_b * src_channels;
    for (dst_ch, sample) in dst.iter_mut().enumerate() {
        let src_ch = dst_ch % src_channels;
        let a = samples.get(off_a + src_ch).copied().unwrap_or(0.0);
        let b = samples.get(off_b + src_ch).copied().unwrap_or(0.0);
        *sample = (a + (b - a) * frac) * volume;
    }
}

// Relaxed ordering is intentional: audio callbacks tolerate stale values for a few
// frames (< 1 ms) without audible artifacts, and stronger ordering would add overhead
// to the hot audio path for no practical benefit.
struct SharedState {
    cursor: AtomicU64,
    volume: AtomicU32,
    playback: AtomicU8,
    looping: AtomicU8,
    total_src_frames: u64,
    dst_rate: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PlaybackState {
    Playing = 0,
    Paused = 1,
    Stopped = 2,
}

impl From<u8> for PlaybackState {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Playing,
            1 => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// Handle to a playing audio stream. Clone + Send — safe to share across threads.
///
/// All control methods (play/pause/stop/volume) operate via lock-free atomics.
/// The underlying audio stream is automatically stopped when every clone is dropped.
#[derive(Clone)]
pub struct AudioHandle {
    state: Arc<SharedState>,
    _drop_guard: Arc<mpsc::Sender<()>>,
}

impl AudioHandle {
    pub fn play(&self) {
        let prev = self.state.playback.load(Ordering::Relaxed);
        if prev == PlaybackState::Stopped as u8 {
            self.state.cursor.store(0, Ordering::Relaxed);
        }
        self.state
            .playback
            .store(PlaybackState::Playing as u8, Ordering::Relaxed);
    }

    pub fn pause(&self) {
        self.state
            .playback
            .store(PlaybackState::Paused as u8, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.state
            .playback
            .store(PlaybackState::Stopped as u8, Ordering::Relaxed);
        self.state.cursor.store(0, Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: f32) {
        self.state
            .volume
            .store(volume.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn volume(&self) -> f32 {
        f32::from_bits(self.state.volume.load(Ordering::Relaxed))
    }

    pub fn set_looping(&self, looping: bool) {
        self.state
            .looping
            .store(u8::from(looping), Ordering::Relaxed);
    }

    pub fn is_looping(&self) -> bool {
        self.state.looping.load(Ordering::Relaxed) != 0
    }

    pub fn state(&self) -> PlaybackState {
        PlaybackState::from(self.state.playback.load(Ordering::Relaxed))
    }

    pub fn position_secs(&self) -> f64 {
        self.state.cursor.load(Ordering::Relaxed) as f64 / self.state.dst_rate as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_mono_no_interpolation() {
        let samples = [0.0f32, 0.5, 1.0];
        let mut dst = [0.0f32; 1];
        fill_frame_lerp(&mut dst, &samples, 1, 2, 0.0, 1, 1.0);
        assert_eq!(dst[0], 0.5);
    }

    #[test]
    fn lerp_mono_half() {
        let samples = [0.0f32, 1.0];
        let mut dst = [0.0f32; 1];
        fill_frame_lerp(&mut dst, &samples, 0, 1, 0.5, 1, 1.0);
        assert!((dst[0] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_stereo() {
        let samples = [0.0f32, 1.0, 1.0, 0.0];
        let mut dst = [0.0f32; 2];
        fill_frame_lerp(&mut dst, &samples, 0, 1, 0.5, 2, 1.0);
        assert!((dst[0] - 0.5).abs() < f32::EPSILON);
        assert!((dst[1] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn lerp_mono_to_stereo() {
        let samples = [0.8f32, 0.2];
        let mut dst = [0.0f32; 2];
        fill_frame_lerp(&mut dst, &samples, 0, 1, 0.0, 1, 1.0);
        assert_eq!(dst[0], 0.8);
        assert_eq!(dst[1], 0.8);
    }

    #[test]
    fn lerp_volume() {
        let samples = [1.0f32];
        let mut dst = [0.0f32; 1];
        fill_frame_lerp(&mut dst, &samples, 0, 0, 0.0, 1, 0.5);
        assert!((dst[0] - 0.5).abs() < f32::EPSILON);
    }
}
