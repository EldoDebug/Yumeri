// Workaround: rustc 1.94 ICE in `check_mod_deathness`
#![allow(dead_code)]

pub(crate) mod clock;
pub(crate) mod decode;
pub(crate) mod demux;
pub(crate) mod error;
pub(crate) mod frame;
pub(crate) mod pixel_format;
pub(crate) mod player;
pub(crate) mod video;

pub use decode::VulkanDeviceInfo;
pub use error::{VideoError, Result};
pub use frame::{GpuFrame, VideoFrame};
pub use pixel_format::VideoPixelFormat;
pub use player::{VideoControl, VideoHandle, VideoPlayer};
pub use video::{Video, VideoCodec};

pub use yumeri_audio::PlaybackState;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_video_path() -> String {
        let manifest = env!("CARGO_MANIFEST_DIR");
        format!("{manifest}/../../test.mp4")
    }

    #[test]
    fn probe_returns_valid_metadata() {
        let path = test_video_path();
        let info = Video::probe(&path).expect("failed to probe");

        assert!(info.width() > 0);
        assert!(info.height() > 0);
        assert!(info.frame_rate() > 0.0);
        assert!(info.duration_secs() > 0.0);
    }

    #[test]
    fn probe_detects_audio() {
        let path = test_video_path();
        let info = Video::probe(&path).expect("failed to probe");
        // Most test videos have audio
        assert!(info.has_audio());
    }

    #[test]
    fn probe_nonexistent_file() {
        let result = Video::probe("nonexistent_video.mp4");
        assert!(result.is_err());
    }

    #[test]
    fn video_player_creates_handle() {
        let path = test_video_path();
        let player = VideoPlayer::new().expect("failed to create player");
        let handle = player.play(&path).expect("failed to play");

        assert!(handle.width() > 0);
        assert!(handle.height() > 0);
        assert!(handle.frame_rate() > 0.0);
        assert_eq!(handle.state(), PlaybackState::Playing);

        handle.stop();
        std::thread::sleep(std::time::Duration::from_millis(50));
        assert_eq!(handle.state(), PlaybackState::Stopped);
    }

    #[test]
    fn video_player_decodes_frames() {
        let path = test_video_path();
        let player = VideoPlayer::new().expect("failed to create player");
        let handle = player.play(&path).expect("failed to play");

        // Wait for decode thread to produce some frames
        std::thread::sleep(std::time::Duration::from_millis(500));

        let frame = handle.next_frame();
        assert!(frame.is_some(), "expected at least one decoded frame");

        let frame = frame.unwrap();
        assert!(frame.width() > 0);
        assert!(frame.height() > 0);
        assert!(frame.pts() >= 0.0);

        handle.stop();
    }

    #[test]
    fn video_handle_pause_resume() {
        let path = test_video_path();
        let player = VideoPlayer::new().expect("failed to create player");
        let handle = player.play(&path).expect("failed to play");

        handle.pause();
        assert_eq!(handle.state(), PlaybackState::Paused);

        handle.play();
        assert_eq!(handle.state(), PlaybackState::Playing);

        handle.stop();
    }
}
