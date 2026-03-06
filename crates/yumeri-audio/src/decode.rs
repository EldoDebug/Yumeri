use std::io::Cursor;
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::Audio;
use crate::error::{AudioError, Result};
use crate::sample_format::SampleFormat;

pub(crate) fn decode_from_path(path: &Path, format: SampleFormat) -> Result<Audio> {
    let file = std::fs::File::open(path).map_err(|e| AudioError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    decode_stream(mss, hint, format)
}

pub(crate) fn decode_from_memory(bytes: &[u8], format: SampleFormat) -> Result<Audio> {
    let cursor = Cursor::new(bytes.to_vec());
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());
    decode_stream(mss, Hint::new(), format)
}

fn decode_stream(
    mss: MediaSourceStream,
    hint: Hint,
    target_format: SampleFormat,
) -> Result<Audio> {
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(AudioError::NoAudioTrack)?;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| AudioError::Decode("unknown sample rate".into()))?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count() as u16)
        .ok_or_else(|| AudioError::Decode("unknown channel count".into()))?;

    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| AudioError::Decode(e.to_string()))?;

    let mut f32_samples: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::IoError(_) | SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(AudioError::Decode(e.to_string())),
        };

        let spec = *decoded.spec();
        let capacity = decoded.capacity() as u64;

        let buf = sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(capacity, spec));

        if (buf.capacity() as u64) < capacity {
            *buf = SampleBuffer::<f32>::new(capacity, spec);
        }

        buf.copy_interleaved_ref(decoded);
        f32_samples.extend_from_slice(buf.samples());
    }

    let data = samples_to_bytes(&f32_samples, target_format);
    Ok(Audio::from_raw(data, sample_rate, channels, target_format))
}

pub(crate) fn convert(audio: &Audio, target: SampleFormat) -> Result<Audio> {
    if audio.format() == target {
        return Ok(audio.clone());
    }

    let f32_samples = bytes_to_f32_samples(audio.data(), audio.format());
    let data = samples_to_bytes(&f32_samples, target);
    Ok(Audio::from_raw(
        data,
        audio.sample_rate(),
        audio.channels(),
        target,
    ))
}

fn samples_to_bytes(samples: &[f32], format: SampleFormat) -> Vec<u8> {
    match format {
        SampleFormat::F32 => {
            let mut bytes = Vec::with_capacity(samples.len() * 4);
            for &s in samples {
                bytes.extend_from_slice(&s.to_le_bytes());
            }
            bytes
        }
        SampleFormat::I16 => {
            let mut bytes = Vec::with_capacity(samples.len() * 2);
            for &s in samples {
                let clamped = s.clamp(-1.0, 1.0);
                let i = (clamped * f32::from(i16::MAX)) as i16;
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            bytes
        }
        SampleFormat::I32 => {
            let mut bytes = Vec::with_capacity(samples.len() * 4);
            for &s in samples {
                let clamped = s.clamp(-1.0, 1.0);
                let i = (f64::from(clamped) * i32::MAX as f64) as i32;
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            bytes
        }
    }
}

pub(crate) fn bytes_to_f32_samples(data: &[u8], format: SampleFormat) -> Vec<f32> {
    match format {
        SampleFormat::F32 => data
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect(),
        SampleFormat::I16 => data
            .chunks_exact(2)
            .map(|chunk| {
                let i = i16::from_le_bytes(chunk.try_into().unwrap());
                f32::from(i) / f32::from(i16::MAX)
            })
            .collect(),
        SampleFormat::I32 => data
            .chunks_exact(4)
            .map(|chunk| {
                let i = i32::from_le_bytes(chunk.try_into().unwrap());
                i as f32 / i32::MAX as f32
            })
            .collect(),
    }
}
