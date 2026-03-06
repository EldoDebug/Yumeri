pub(crate) mod audio;
pub(crate) mod cache;
pub(crate) mod decode;
pub(crate) mod error;
pub(crate) mod player;
pub(crate) mod sample_format;

pub use audio::Audio;
pub use cache::{AudioCache, AudioId, LoadStatus};
pub use error::{AudioError, Result};
pub use player::{AudioHandle, AudioPlayer, PlaybackState};
pub use sample_format::SampleFormat;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wav(
        sample_rate: u32,
        channels: u16,
        format_code: u16,
        bits_per_sample: u16,
        sample_data: &[u8],
    ) -> Vec<u8> {
        let data_size = sample_data.len() as u32;
        let block_align = channels * (bits_per_sample / 8);
        let byte_rate = sample_rate * block_align as u32;
        let riff_size = 4 + 24 + 8 + data_size;

        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&riff_size.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&format_code.to_le_bytes());
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        buf.extend_from_slice(sample_data);
        buf
    }

    fn make_wav_f32(sample_rate: u32, channels: u16, samples: &[f32]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        make_wav(sample_rate, channels, 3, 32, &data)
    }

    fn make_wav_i16(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        make_wav(sample_rate, channels, 1, 16, &data)
    }

    #[test]
    fn sample_format_bytes_per_sample() {
        assert_eq!(SampleFormat::F32.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::I16.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::I32.bytes_per_sample(), 4);
    }

    #[test]
    fn sample_format_default_is_f32() {
        assert_eq!(SampleFormat::default(), SampleFormat::F32);
    }

    #[test]
    fn decode_wav_f32_mono() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode(&wav).unwrap();

        assert_eq!(audio.sample_rate(), 44100);
        assert_eq!(audio.channels(), 1);
        assert_eq!(audio.format(), SampleFormat::F32);
        assert_eq!(audio.frame_count(), 4);
        assert_eq!(audio.byte_len(), 16);

        let decoded: Vec<f32> = audio
            .data()
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(decoded, samples);
    }

    #[test]
    fn decode_wav_i16_stereo() {
        let samples = vec![1000i16, -1000, 16000, -16000];
        let wav = make_wav_i16(48000, 2, &samples);
        let audio = Audio::decode(&wav).unwrap();

        assert_eq!(audio.sample_rate(), 48000);
        assert_eq!(audio.channels(), 2);
        assert_eq!(audio.format(), SampleFormat::F32);
        assert_eq!(audio.frame_count(), 2);
    }

    #[test]
    fn decode_with_i16_format() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode_with(&wav, SampleFormat::I16).unwrap();

        assert_eq!(audio.format(), SampleFormat::I16);
        assert_eq!(audio.frame_count(), 4);
        assert_eq!(audio.byte_len(), 8);
    }

    #[test]
    fn convert_f32_to_i16_and_back() {
        let samples = vec![0.0f32, 0.5, -0.5, 1.0];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode(&wav).unwrap();

        let i16_audio = audio.convert_to(SampleFormat::I16).unwrap();
        assert_eq!(i16_audio.format(), SampleFormat::I16);
        assert_eq!(i16_audio.frame_count(), 4);

        let back = i16_audio.convert_to(SampleFormat::F32).unwrap();
        assert_eq!(back.format(), SampleFormat::F32);

        let original: Vec<f32> = audio
            .data()
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        let roundtrip: Vec<f32> = back
            .data()
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect();

        for (a, b) in original.iter().zip(&roundtrip) {
            assert!((a - b).abs() < 0.001, "expected ~{a}, got {b}");
        }
    }

    #[test]
    fn convert_same_format_returns_clone() {
        let samples = vec![0.0f32, 0.5];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode(&wav).unwrap();
        let converted = audio.convert_to(SampleFormat::F32).unwrap();
        assert_eq!(audio.data(), converted.data());
    }

    #[test]
    fn audio_duration() {
        let samples = vec![0.0f32; 44100];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode(&wav).unwrap();

        assert!((audio.duration_secs() - 1.0).abs() < f64::EPSILON);
        assert_eq!(audio.duration(), std::time::Duration::from_secs(1));
    }

    #[test]
    fn into_data_consumes_audio() {
        let samples = vec![0.25f32, 0.75];
        let wav = make_wav_f32(44100, 1, &samples);
        let audio = Audio::decode(&wav).unwrap();
        let data = audio.into_data();
        assert_eq!(data.len(), 8);
    }

    #[test]
    fn load_from_file() {
        let samples = vec![0.0f32, 1.0, -1.0];
        let wav = make_wav_f32(44100, 1, &samples);

        let dir = std::env::temp_dir().join("yumeri_audio_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.wav");
        std::fs::write(&path, &wav).unwrap();

        let audio = Audio::load(&path).unwrap();
        assert_eq!(audio.sample_rate(), 44100);
        assert_eq!(audio.channels(), 1);
        assert_eq!(audio.frame_count(), 3);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn load_nonexistent_file_returns_io_error() {
        let result = Audio::load("nonexistent.wav");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AudioError::Io { .. }));
    }

    #[test]
    fn decode_invalid_bytes_returns_decode_error() {
        let result = Audio::decode(&[0, 1, 2, 3]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AudioError::Decode(_)));
    }

    #[test]
    fn audio_cache_dedup() {
        let samples = vec![0.0f32; 100];
        let wav = make_wav_f32(44100, 1, &samples);

        let dir = std::env::temp_dir().join("yumeri_audio_cache_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("dedup.wav");
        std::fs::write(&path, &wav).unwrap();

        let mut cache = AudioCache::new();
        let id1 = cache.load(&path);
        let id2 = cache.load(&path);
        assert_eq!(id1, id2);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    #[test]
    fn audio_cache_insert_and_get() {
        let audio = Audio::from_raw(vec![0; 16], 44100, 1, SampleFormat::F32);
        let mut cache = AudioCache::new();
        let id = cache.insert(audio);

        assert_eq!(cache.status(id), LoadStatus::Ready);
        let retrieved = cache.get(id).unwrap();
        assert_eq!(retrieved.sample_rate(), 44100);
    }

    #[test]
    fn audio_cache_remove() {
        let audio = Audio::from_raw(vec![0; 16], 44100, 1, SampleFormat::F32);
        let mut cache = AudioCache::new();
        let id = cache.insert(audio);

        cache.remove(id);
        assert!(cache.get(id).is_none());
        assert_eq!(cache.status(id), LoadStatus::Failed);
    }

    #[test]
    fn audio_cache_async_load() {
        let samples = vec![0.0f32; 100];
        let wav = make_wav_f32(44100, 1, &samples);

        let dir = std::env::temp_dir().join("yumeri_audio_async_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("async.wav");
        std::fs::write(&path, &wav).unwrap();

        let mut cache = AudioCache::new();
        let id = cache.load(&path);
        assert_eq!(cache.status(id), LoadStatus::Loading);

        std::thread::sleep(std::time::Duration::from_millis(500));
        cache.process_pending();

        assert_eq!(cache.status(id), LoadStatus::Ready);
        let audio = cache.get(id).unwrap();
        assert_eq!(audio.sample_rate(), 44100);
        assert_eq!(audio.frame_count(), 100);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }
}
