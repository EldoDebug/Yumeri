use std::io::{self, Write};

use yumeri_video::{PlaybackState, Video, VideoPlayer};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test.mp4".to_string());

    println!("Probing: {path}");
    let info = Video::probe(&path).expect("failed to probe video");
    println!(
        "  {}x{}, {:.2} fps, {:.2}s, codec: {:?}, audio: {}",
        info.width(),
        info.height(),
        info.frame_rate(),
        info.duration_secs(),
        info.codec(),
        info.has_audio(),
    );

    let player = VideoPlayer::new().expect("failed to create player");
    let handle = player.play(&path).expect("failed to play video");
    println!(
        "  Output: {}x{} @ {:.2} fps",
        handle.width(),
        handle.height(),
        handle.frame_rate(),
    );

    println!("Playing. Draining decoded frames...");
    println!("Press Ctrl+C to stop.\n");

    let mut frame_count = 0u64;
    while handle.state() != PlaybackState::Stopped {
        if let Some(frame) = handle.next_frame() {
            frame_count += 1;
            if frame_count % 30 == 1 {
                print!(
                    "\r  Frame {frame_count}: {}x{} pts={:.3}s pos={:.3}s",
                    frame.width(),
                    frame.height(),
                    frame.pts(),
                    handle.position_secs(),
                );
                io::stdout().flush().unwrap();
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    println!("\n  Total frames decoded: {frame_count}");
    println!("Stopped.");
}
