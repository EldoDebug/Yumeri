use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use crate::effect::AudioEffect;

const MAX_CHANNELS: usize = 8;
const DENORMAL_PREVENTION: f32 = 1e-25;

struct SharedParams {
    cutoff_hz: AtomicU32,
    q: AtomicU32,
}

/// Handle for controlling a `LowPass` effect from any thread.
pub struct LowPassHandle {
    params: Arc<SharedParams>,
}

impl LowPassHandle {
    pub fn set_cutoff(&self, hz: f32) {
        let hz = if hz.is_finite() { hz.max(1.0) } else { 1.0 };
        self.params.cutoff_hz.store(hz.to_bits(), Ordering::Relaxed);
    }

    pub fn cutoff(&self) -> f32 {
        f32::from_bits(self.params.cutoff_hz.load(Ordering::Relaxed))
    }

    pub fn set_q(&self, q: f32) {
        let q = if q.is_finite() { q.max(0.01) } else { 0.707 };
        self.params.q.store(q.to_bits(), Ordering::Relaxed);
    }

    pub fn q(&self) -> f32 {
        f32::from_bits(self.params.q.load(Ordering::Relaxed))
    }
}

#[derive(Clone, Copy)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Clone, Copy, Default)]
struct BiquadState {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

/// Biquad low-pass filter based on Robert Bristow-Johnson's Audio EQ Cookbook.
pub struct LowPass {
    params: Arc<SharedParams>,
    coeffs: BiquadCoeffs,
    cached_cutoff_bits: u32,
    cached_q_bits: u32,
    cached_sample_rate: u32,
    states: [BiquadState; MAX_CHANNELS],
    anti_denormal: f32,
}

impl LowPass {
    pub fn new(cutoff_hz: f32, q: f32) -> (Self, LowPassHandle) {
        let params = Arc::new(SharedParams {
            cutoff_hz: AtomicU32::new(cutoff_hz.to_bits()),
            q: AtomicU32::new(q.to_bits()),
        });

        let effect = Self {
            params: Arc::clone(&params),
            coeffs: BiquadCoeffs {
                b0: 1.0,
                b1: 0.0,
                b2: 0.0,
                a1: 0.0,
                a2: 0.0,
            },
            cached_cutoff_bits: 0,
            cached_q_bits: 0,
            cached_sample_rate: 0,
            states: [BiquadState::default(); MAX_CHANNELS],
            anti_denormal: DENORMAL_PREVENTION,
        };

        let handle = LowPassHandle { params };
        (effect, handle)
    }

    fn update_coeffs(&mut self, sample_rate: u32) {
        let cutoff_bits = self.params.cutoff_hz.load(Ordering::Relaxed);
        let q_bits = self.params.q.load(Ordering::Relaxed);

        if cutoff_bits == self.cached_cutoff_bits
            && q_bits == self.cached_q_bits
            && sample_rate == self.cached_sample_rate
        {
            return;
        }

        self.cached_cutoff_bits = cutoff_bits;
        self.cached_q_bits = q_bits;
        self.cached_sample_rate = sample_rate;

        let fs = sample_rate as f32;
        let fc = f32::from_bits(cutoff_bits).clamp(1.0, fs * 0.499);
        let q = f32::from_bits(q_bits).max(0.01);

        let w0 = 2.0 * std::f32::consts::PI * fc / fs;
        let (sin_w0, cos_w0) = w0.sin_cos();
        let alpha = sin_w0 / (2.0 * q);

        let b0 = (1.0 - cos_w0) / 2.0;
        let b1 = 1.0 - cos_w0;
        let b2 = (1.0 - cos_w0) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        let inv_a0 = 1.0 / a0;
        self.coeffs = BiquadCoeffs {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
        };
    }
}

impl AudioEffect for LowPass {
    fn process(&mut self, samples: &mut [f32], channels: usize, sample_rate: u32) {
        self.update_coeffs(sample_rate);

        let channels = channels.min(MAX_CHANNELS);
        let c = &self.coeffs;

        for frame in samples.chunks_mut(channels) {
            for (ch, sample) in frame.iter_mut().enumerate() {
                let s = &mut self.states[ch];
                let x0 = *sample;
                let y0 = c.b0 * x0 + c.b1 * s.x1 + c.b2 * s.x2
                    - c.a1 * s.y1
                    - c.a2 * s.y2
                    + self.anti_denormal;
                self.anti_denormal = -self.anti_denormal;

                s.x2 = s.x1;
                s.x1 = x0;
                s.y2 = s.y1;
                s.y1 = y0;

                *sample = y0;
            }
        }
    }

    fn reset(&mut self) {
        self.states = [BiquadState::default(); MAX_CHANNELS];
        self.anti_denormal = DENORMAL_PREVENTION;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dc_passthrough() {
        let (mut lp, _handle) = LowPass::new(1000.0, 0.707);
        let mut samples = vec![1.0f32; 44100];
        lp.process(&mut samples, 1, 44100);
        let last = samples[samples.len() - 1];
        assert!(
            (last - 1.0).abs() < 0.01,
            "DC should pass through LPF, got {last}"
        );
    }

    #[test]
    fn high_freq_attenuation() {
        let (mut lp, _handle) = LowPass::new(1000.0, 0.707);
        let sample_rate = 44100u32;
        let freq = 10000.0f32;
        let len = 44100;

        let mut samples: Vec<f32> = (0..len)
            .map(|i| {
                (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin()
            })
            .collect();

        let input_rms: f32 =
            (samples.iter().map(|s| s * s).sum::<f32>() / len as f32).sqrt();
        lp.process(&mut samples, 1, sample_rate);
        // Skip first 1000 samples for filter settling
        let settled = &samples[1000..];
        let output_rms: f32 =
            (settled.iter().map(|s| s * s).sum::<f32>() / settled.len() as f32).sqrt();

        assert!(
            output_rms < input_rms * 0.1,
            "10kHz should be strongly attenuated: input_rms={input_rms}, output_rms={output_rms}"
        );
    }

    #[test]
    fn channel_independence() {
        let (mut lp, _handle) = LowPass::new(1000.0, 0.707);
        let sample_rate = 44100u32;
        let len = 44100;
        let freq_low = 100.0f32;
        let freq_high = 10000.0f32;

        // Interleaved stereo: ch0 = 100Hz, ch1 = 10kHz
        let mut samples: Vec<f32> = (0..len)
            .flat_map(|i| {
                let t = i as f32 / sample_rate as f32;
                [
                    (2.0 * std::f32::consts::PI * freq_low * t).sin(),
                    (2.0 * std::f32::consts::PI * freq_high * t).sin(),
                ]
            })
            .collect();

        lp.process(&mut samples, 2, sample_rate);

        // Skip first 1000 frames (2000 samples) for settling
        let settled = &samples[2000..];
        let ch0_rms: f32 = {
            let vals: Vec<f32> = settled.iter().step_by(2).copied().collect();
            (vals.iter().map(|s| s * s).sum::<f32>() / vals.len() as f32).sqrt()
        };
        let ch1_rms: f32 = {
            let vals: Vec<f32> = settled.iter().skip(1).step_by(2).copied().collect();
            (vals.iter().map(|s| s * s).sum::<f32>() / vals.len() as f32).sqrt()
        };

        assert!(
            ch0_rms > 0.5,
            "100Hz ch0 should pass through, got rms={ch0_rms}"
        );
        assert!(
            ch1_rms < 0.1,
            "10kHz ch1 should be attenuated, got rms={ch1_rms}"
        );
    }

    #[test]
    fn reset_clears_state() {
        let (mut lp, _handle) = LowPass::new(1000.0, 0.707);
        let mut warmup = vec![1.0f32; 100];
        lp.process(&mut warmup, 1, 44100);
        lp.reset();

        let (mut lp2, _) = LowPass::new(1000.0, 0.707);

        let mut samples_a = vec![1.0f32; 100];
        let mut samples_b = vec![1.0f32; 100];
        lp.process(&mut samples_a, 1, 44100);
        lp2.process(&mut samples_b, 1, 44100);

        assert_eq!(samples_a, samples_b);
    }

    #[test]
    fn nyquist_stability() {
        let (mut lp, _handle) = LowPass::new(22050.0, 0.707);
        let mut samples = vec![1.0f32; 44100];
        lp.process(&mut samples, 1, 44100);

        for s in &samples {
            assert!(s.is_finite(), "output should not contain NaN/Inf");
        }
    }

    #[test]
    fn parameter_change_stays_finite() {
        let (mut lp, handle) = LowPass::new(1000.0, 0.707);
        let mut samples = vec![0.5f32; 1000];
        lp.process(&mut samples, 1, 44100);

        handle.set_cutoff(500.0);
        handle.set_q(2.0);

        let mut samples2 = vec![0.5f32; 1000];
        lp.process(&mut samples2, 1, 44100);

        for s in &samples2 {
            assert!(s.is_finite(), "output should be finite after parameter change");
        }
    }

    #[test]
    fn handle_round_trip() {
        let (_lp, handle) = LowPass::new(1000.0, 0.707);

        assert!((handle.cutoff() - 1000.0).abs() < f32::EPSILON);
        assert!((handle.q() - 0.707).abs() < f32::EPSILON);

        handle.set_cutoff(500.0);
        handle.set_q(2.0);

        assert!((handle.cutoff() - 500.0).abs() < f32::EPSILON);
        assert!((handle.q() - 2.0).abs() < f32::EPSILON);
    }
}
