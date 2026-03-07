use std::thread;
use std::time::Duration;

use yumeri_audio::{Audio, AudioPlayer, EffectChain, LowPass};

fn main() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test.mp3");
    let audio = Audio::load(&path).expect("failed to load test.mp3");
    println!(
        "Loaded: {}Hz, {}ch, {} frames ({:.1}s)",
        audio.sample_rate(),
        audio.channels(),
        audio.frame_count(),
        audio.duration_secs(),
    );

    let player = AudioPlayer::new().expect("failed to create audio player");
    println!(
        "Output device: {}Hz, {}ch",
        player.sample_rate(),
        player.channels(),
    );

    let (low_pass, lp_handle) = LowPass::new(1000.0, 0.707);
    let chain = EffectChain::new().with(low_pass);
    let handle = player
        .play_with_effects(&audio, 0.8, false, chain)
        .expect("failed to play");

    println!("\n--- Low-Pass Filter Demo ---");
    println!("Playing with cutoff=1000Hz, Q=0.707\n");

    let steps: &[(f32, f32, &str)] = &[
        (1000.0, 0.707, "1000Hz (initial)"),
        (500.0, 0.707, "500Hz - more muffled"),
        (200.0, 0.707, "200Hz - very muffled"),
        (2000.0, 0.707, "2000Hz - opening up"),
        (5000.0, 0.707, "5000Hz - mostly open"),
        (500.0, 5.0, "500Hz Q=5.0 - resonant"),
        (1000.0, 0.707, "1000Hz - back to start"),
    ];

    for &(cutoff, q, label) in steps {
        thread::sleep(Duration::from_secs(2));

        if handle.state() != yumeri_audio::PlaybackState::Playing {
            println!("Playback ended.");
            break;
        }

        lp_handle.set_cutoff(cutoff);
        lp_handle.set_q(q);
        println!(
            "[{:5.1}s] cutoff={cutoff:>5.0}Hz  Q={q:.3}  -- {label}",
            handle.position_secs(),
        );
    }

    // Let it play for a few more seconds
    println!("\nPlaying remaining...");
    while handle.state() == yumeri_audio::PlaybackState::Playing {
        thread::sleep(Duration::from_secs(1));
    }
    println!("Done.");
}
