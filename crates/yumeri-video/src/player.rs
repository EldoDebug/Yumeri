use std::path::Path;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicU8, Ordering};
use std::sync::{mpsc, Arc};

use rsmpeg::avcodec::AVCodecContext;
use rsmpeg::avutil::AVFrame;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use yumeri_audio::{Audio, AudioHandle, AudioPlayer, PlaybackState, SampleFormat};

use crate::clock::PresentationClock;
use crate::decode::{self, VulkanDeviceInfo};
use crate::demux::{create_decoder, DemuxPacket, Demuxer};
use crate::error::{Result, VideoError};
use crate::frame::VideoFrame;

const FRAME_QUEUE_CAPACITY: usize = 4;
const SEEK_NONE: i64 = -1;

/// Factory for creating video playback sessions.
pub struct VideoPlayer {
    vulkan_info: Option<VulkanDeviceInfo>,
}

impl VideoPlayer {
    pub fn new() -> Result<Self> {
        Ok(Self { vulkan_info: None })
    }

    /// Create a player that uses Vulkan hardware-accelerated decoding.
    /// Falls back to software decode if hwaccel is unavailable.
    pub fn with_vulkan(info: VulkanDeviceInfo) -> Result<Self> {
        Ok(Self {
            vulkan_info: Some(info),
        })
    }

    /// Start playing a video file. Returns a handle for controlling playback.
    pub fn play(&self, path: impl AsRef<Path>) -> Result<VideoHandle> {
        self.play_with(path, 1.0, false)
    }

    /// Start playing with volume and loop settings.
    pub fn play_with(
        &self,
        path: impl AsRef<Path>,
        volume: f32,
        looping: bool,
    ) -> Result<VideoHandle> {
        let path = path.as_ref().to_path_buf();

        let mut demuxer = Demuxer::open(&path)?;
        let width = demuxer.video_width();
        let height = demuxer.video_height();
        let frame_rate = demuxer.video_frame_rate();
        let duration_us = (demuxer.duration_secs() * 1_000_000.0) as u64;

        // Pre-decode audio track if available
        let audio_handle = if demuxer.has_audio() {
            match pre_decode_audio(&mut demuxer, volume) {
                Ok(ah) => Some(ah),
                Err(e) => {
                    log::warn!("Failed to decode audio: {e}");
                    // Seek back to start in case we partially consumed packets
                    let _ = demuxer.seek(0);
                    None
                }
            }
        } else {
            None
        };

        let state = Arc::new(SharedState {
            playback: AtomicU8::new(PlaybackState::Playing as u8),
            looping: AtomicU8::new(u8::from(looping)),
            position_us: AtomicU64::new(0),
            duration_us,
            width,
            height,
            frame_rate,
            seek_target: AtomicI64::new(SEEK_NONE),
        });

        let (frame_tx, frame_rx) = mpsc::sync_channel::<VideoFrame>(FRAME_QUEUE_CAPACITY);
        let (drop_tx, drop_rx) = mpsc::channel::<()>();

        let thread_state = Arc::clone(&state);
        let thread_audio = audio_handle.clone();
        let vulkan_info = self.vulkan_info.clone();

        std::thread::Builder::new()
            .name("yumeri-video-decode".into())
            .spawn(move || {
                decode_thread(
                    demuxer,
                    thread_audio,
                    thread_state,
                    frame_tx,
                    drop_rx,
                    vulkan_info,
                );
            })
            .map_err(|e| VideoError::Playback(format!("failed to spawn decode thread: {e}")))?;

        Ok(VideoHandle {
            control: VideoControl {
                state,
                audio_handle,
            },
            frame_rx,
            _drop_guard: Arc::new(drop_tx),
        })
    }
}

/// Pre-decode the entire audio track from the demuxer into a PCM buffer,
/// then seek the demuxer back to 0 so the video decode thread can start fresh.
fn pre_decode_audio(demuxer: &mut Demuxer, volume: f32) -> Result<AudioHandle> {
    let audio_codecpar = demuxer
        .audio_codecpar()
        .ok_or_else(|| VideoError::Playback("no audio track".into()))?
        .clone();

    let mut audio_decoder = create_decoder(&audio_codecpar, "audio")?;

    let sample_rate = audio_codecpar.sample_rate as u32;
    let channels = audio_codecpar.ch_layout.nb_channels as u16;

    if sample_rate == 0 || channels == 0 {
        return Err(VideoError::Playback("invalid audio parameters".into()));
    }

    let mut all_samples: Vec<f32> = Vec::new();

    // Read all packets, decode only audio ones
    loop {
        match demuxer.read_packet()? {
            DemuxPacket::Audio(pkt) => {
                audio_decoder
                    .send_packet(Some(&pkt))
                    .map_err(|e| VideoError::Decode(format!("audio send_packet: {e}")))?;
                drain_audio_frames(&mut audio_decoder, channels as usize, &mut all_samples);
            }
            DemuxPacket::Video(_) => continue,
            DemuxPacket::Eof => break,
        }
    }

    // Flush remaining frames from the decoder
    let _ = audio_decoder.send_packet(None);
    drain_audio_frames(&mut audio_decoder, channels as usize, &mut all_samples);

    // Seek back to start for video decode
    demuxer.seek(0)?;

    if all_samples.is_empty() {
        return Err(VideoError::Playback("no audio samples decoded".into()));
    }

    log::info!(
        "Pre-decoded {} audio samples ({:.1}s, {}ch, {}Hz)",
        all_samples.len(),
        all_samples.len() as f64 / (sample_rate as f64 * channels as f64),
        channels,
        sample_rate,
    );

    // Reinterpret Vec<f32> as Vec<u8> without copying (f32 is already LE on LE platforms)
    let (ptr, len, cap) = {
        let mut samples = std::mem::ManuallyDrop::new(all_samples);
        (samples.as_mut_ptr(), samples.len(), samples.capacity())
    };
    let data = unsafe { Vec::from_raw_parts(ptr as *mut u8, len * 4, cap * 4) };
    let audio = Audio::from_raw(data, sample_rate, channels, SampleFormat::F32);

    let player = AudioPlayer::new().map_err(|e| VideoError::Playback(e.to_string()))?;
    let handle = player
        .play_with(&audio, volume, false)
        .map_err(|e| VideoError::Playback(e.to_string()))?;

    Ok(handle)
}

fn drain_audio_frames(decoder: &mut AVCodecContext, channels: usize, output: &mut Vec<f32>) {
    loop {
        match decoder.receive_frame() {
            Ok(frame) => convert_audio_frame(&frame, channels, output),
            Err(RsmpegError::DecoderDrainError | RsmpegError::DecoderFlushedError) => break,
            Err(e) => {
                log::warn!("Audio decode error: {e}");
                break;
            }
        }
    }
}

/// Convert a decoded AVFrame to f32 interleaved samples and append to output.
fn convert_audio_frame(frame: &AVFrame, channels: usize, output: &mut Vec<f32>) {
    let nb_samples = unsafe { (*frame.as_ptr()).nb_samples } as usize;
    let format = unsafe { (*frame.as_ptr()).format };

    if nb_samples == 0 || channels == 0 {
        return;
    }

    output.reserve(nb_samples * channels);

    // Use extended_data instead of data[] for planar formats — data[] is limited
    // to 8 entries (AV_NUM_DATA_POINTERS) and would cause UB with > 8 channels.
    let planes = unsafe { (*frame.as_ptr()).extended_data };

    match format {
        x if x == ffi::AV_SAMPLE_FMT_FLTP as i32 => {
            for i in 0..nb_samples {
                for ch in 0..channels {
                    let ptr = unsafe { *planes.add(ch) as *const f32 };
                    output.push(unsafe { *ptr.add(i) });
                }
            }
        }
        x if x == ffi::AV_SAMPLE_FMT_FLT as i32 => {
            let ptr = unsafe { *planes as *const f32 };
            let slice = unsafe { std::slice::from_raw_parts(ptr, nb_samples * channels) };
            output.extend_from_slice(slice);
        }
        x if x == ffi::AV_SAMPLE_FMT_S16P as i32 => {
            for i in 0..nb_samples {
                for ch in 0..channels {
                    let ptr = unsafe { *planes.add(ch) as *const i16 };
                    output.push(unsafe { *ptr.add(i) } as f32 / 32768.0);
                }
            }
        }
        x if x == ffi::AV_SAMPLE_FMT_S16 as i32 => {
            let ptr = unsafe { *planes as *const i16 };
            let slice = unsafe { std::slice::from_raw_parts(ptr, nb_samples * channels) };
            output.extend(slice.iter().map(|&s| s as f32 / 32768.0));
        }
        x if x == ffi::AV_SAMPLE_FMT_S32P as i32 => {
            for i in 0..nb_samples {
                for ch in 0..channels {
                    let ptr = unsafe { *planes.add(ch) as *const i32 };
                    output.push(unsafe { *ptr.add(i) } as f32 / 2_147_483_648.0);
                }
            }
        }
        x if x == ffi::AV_SAMPLE_FMT_S32 as i32 => {
            let ptr = unsafe { *planes as *const i32 };
            let slice = unsafe { std::slice::from_raw_parts(ptr, nb_samples * channels) };
            output.extend(slice.iter().map(|&s| s as f32 / 2_147_483_648.0));
        }
        _ => {
            log::warn!("Unsupported audio sample format: {format}, filling with silence");
            output.extend(std::iter::repeat_n(0.0f32, nb_samples * channels));
        }
    }
}

/// Receive all available decoded frames and send them to the renderer.
/// Returns true if the frame channel is disconnected (handle dropped).
fn drain_decoded_frames(
    decoder: &mut Box<dyn decode::DecoderBackend>,
    state: &SharedState,
    clock: &PresentationClock,
    frame_interval: f64,
    frame_tx: &mpsc::SyncSender<VideoFrame>,
) -> bool {
    loop {
        match decoder.decode_next() {
            Ok(Some(frame)) => {
                let pts = frame.pts();
                state
                    .position_us
                    .store((pts * 1_000_000.0) as u64, Ordering::Relaxed);

                // A/V sync: skip frames that are too late
                let clock_time = clock.current_time();
                if pts < clock_time - frame_interval {
                    continue;
                }

                // Wait if frame is too early
                let ahead = pts - clock_time;
                if ahead > 0.001 {
                    let sleep_ms = ((ahead - 0.001) * 1000.0) as u64;
                    if sleep_ms > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(sleep_ms.min(100)));
                    }
                }

                if frame_tx.send(frame).is_err() {
                    return true;
                }
            }
            Ok(None) => return false,
            Err(e) => {
                log::error!("Video decode error: {e}");
                return false;
            }
        }
    }
}

fn decode_thread(
    mut demuxer: Demuxer,
    audio_handle: Option<AudioHandle>,
    state: Arc<SharedState>,
    frame_tx: mpsc::SyncSender<VideoFrame>,
    drop_rx: mpsc::Receiver<()>,
    vulkan_info: Option<VulkanDeviceInfo>,
) {
    let video_time_base = demuxer.video_time_base();
    let video_codecpar = demuxer.video_codecpar().clone();

    let mut video_decoder =
        match decode::create_decoder(&video_codecpar, video_time_base, vulkan_info.as_ref()) {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to create video decoder: {e}");
            return;
        }
    };

    let mut clock = PresentationClock::new(audio_handle.clone());
    let frame_interval = 1.0 / state.frame_rate.max(1.0);
    let mut was_paused = false;
    let mut eof_reached = false;

    loop {
        // Check if all handles are dropped (sender side disconnected)
        if matches!(drop_rx.try_recv(), Err(mpsc::TryRecvError::Disconnected)) {
            break;
        }

        let playback = state.playback.load(Ordering::Relaxed);

        if playback == PlaybackState::Stopped as u8 || playback == PlaybackState::Paused as u8 {
            if !was_paused {
                clock.pause();
                was_paused = true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        }

        // Transitioning back to Playing
        if was_paused {
            clock.resume();
            was_paused = false;
        }

        // Restart from beginning after EOF + play()
        if eof_reached {
            if let Err(e) = demuxer.seek(0) {
                log::error!("Restart seek failed: {e}");
                break;
            }
            video_decoder.flush();
            clock.reset(0.0);
            state.position_us.store(0, Ordering::Relaxed);
            eof_reached = false;
        }

        // Handle seek requests
        let seek_us = state.seek_target.swap(SEEK_NONE, Ordering::Relaxed);
        if seek_us != SEEK_NONE {
            let clamped = seek_us.max(0);
            if let Err(e) = demuxer.seek(clamped) {
                log::error!("Seek failed: {e}");
            } else {
                video_decoder.flush();
                clock.reset(clamped as f64 / 1_000_000.0);
                state.position_us.store(clamped as u64, Ordering::Relaxed);
            }
            continue;
        }

        match demuxer.read_packet() {
            Ok(DemuxPacket::Video(pkt)) => {
                // Try send; on EAGAIN, drain frames first, then retry once.
                if let Err(e) = video_decoder.send_packet(&pkt) {
                    if drain_decoded_frames(
                        &mut video_decoder,
                        &state,
                        &clock,
                        frame_interval,
                        &frame_tx,
                    ) {
                        return;
                    }
                    if let Err(e2) = video_decoder.send_packet(&pkt) {
                        log::error!("Failed to send video packet (after drain): {e2} (first: {e})");
                        continue;
                    }
                }

                if drain_decoded_frames(
                    &mut video_decoder,
                    &state,
                    &clock,
                    frame_interval,
                    &frame_tx,
                ) {
                    return;
                }
            }
            Ok(DemuxPacket::Audio(_pkt)) => {
                // Audio is pre-decoded before thread start; skip audio packets here.
            }
            Ok(DemuxPacket::Eof) => {
                let _ = video_decoder.send_eof();
                if drain_decoded_frames(
                    &mut video_decoder,
                    &state,
                    &clock,
                    frame_interval,
                    &frame_tx,
                ) {
                    return;
                }

                if state.looping.load(Ordering::Relaxed) != 0 {
                    if let Err(e) = demuxer.seek(0) {
                        log::error!("Loop seek failed: {e}");
                        break;
                    }
                    video_decoder.flush();
                    clock.reset(0.0);
                    state.position_us.store(0, Ordering::Relaxed);
                } else {
                    state
                        .playback
                        .store(PlaybackState::Stopped as u8, Ordering::Relaxed);
                    eof_reached = true;
                    // Don't break — thread stays alive so play() can restart
                }
            }
            Err(e) => {
                log::error!("Demux error: {e}");
                break;
            }
        }
    }
}

// Relaxed ordering: video frame delivery tolerates stale values for a few frames
// without visual artifacts, similar to the audio crate's approach.
struct SharedState {
    playback: AtomicU8,
    looping: AtomicU8,
    position_us: AtomicU64,
    duration_us: u64,
    width: u32,
    height: u32,
    frame_rate: f64,
    seek_target: AtomicI64,
}

/// Handle to a playing video.
///
/// Playback control methods are available via `Deref<Target = VideoControl>`.
/// The decode thread stops automatically when this handle is dropped.
pub struct VideoHandle {
    control: VideoControl,
    frame_rx: mpsc::Receiver<VideoFrame>,
    _drop_guard: Arc<mpsc::Sender<()>>,
}

/// Lightweight, cloneable handle for controlling video playback (no frame access).
#[derive(Clone)]
pub struct VideoControl {
    state: Arc<SharedState>,
    audio_handle: Option<AudioHandle>,
}

impl std::ops::Deref for VideoHandle {
    type Target = VideoControl;
    fn deref(&self) -> &VideoControl {
        &self.control
    }
}

impl VideoHandle {
    /// Get a cloneable control handle (no frame access).
    pub fn control(&self) -> VideoControl {
        self.control.clone()
    }

    /// Get the next decoded frame (non-blocking). Returns None if no frame is ready.
    pub fn next_frame(&self) -> Option<VideoFrame> {
        self.frame_rx.try_recv().ok()
    }

    /// Access the associated audio handle, if the video has an audio track.
    pub fn audio(&self) -> Option<&AudioHandle> {
        self.control.audio_handle.as_ref()
    }
}

impl VideoControl {
    pub fn play(&self) {
        let prev = self.state.playback.load(Ordering::Relaxed);
        if prev == PlaybackState::Stopped as u8 {
            self.state.position_us.store(0, Ordering::Relaxed);
        }
        self.state
            .playback
            .store(PlaybackState::Playing as u8, Ordering::Relaxed);
        if let Some(ref ah) = self.audio_handle {
            ah.play();
        }
    }

    pub fn pause(&self) {
        self.state
            .playback
            .store(PlaybackState::Paused as u8, Ordering::Relaxed);
        if let Some(ref ah) = self.audio_handle {
            ah.pause();
        }
    }

    pub fn stop(&self) {
        self.state
            .playback
            .store(PlaybackState::Stopped as u8, Ordering::Relaxed);
        self.state.position_us.store(0, Ordering::Relaxed);
        if let Some(ref ah) = self.audio_handle {
            ah.stop();
        }
    }

    pub fn seek(&self, secs: f64) {
        let us = (secs.max(0.0) * 1_000_000.0) as i64;
        self.state.seek_target.store(us, Ordering::Relaxed);
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
        self.state.position_us.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    pub fn duration_secs(&self) -> f64 {
        self.state.duration_us as f64 / 1_000_000.0
    }

    pub fn width(&self) -> u32 {
        self.state.width
    }

    pub fn height(&self) -> u32 {
        self.state.height
    }

    pub fn frame_rate(&self) -> f64 {
        self.state.frame_rate
    }
}
