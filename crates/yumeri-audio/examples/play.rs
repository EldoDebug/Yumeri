use std::io::{self, Write};

use yumeri_audio::{Audio, AudioPlayer};

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test.mp3".to_string());

    println!("Loading: {path}");
    let audio = Audio::load(&path).expect("failed to load audio");
    println!(
        "  Sample rate: {} Hz, Channels: {}, Duration: {:.2}s",
        audio.sample_rate(),
        audio.channels(),
        audio.duration_secs(),
    );

    let player = AudioPlayer::new().expect("failed to create audio player");
    println!(
        "  Output: {} Hz, {} ch",
        player.sample_rate(),
        player.channels(),
    );

    let handle = player.play(&audio).expect("failed to play audio");

    println!("Playing. Press Enter to stop.");
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    handle.stop();
    println!("Stopped.");
}
