pub trait AudioEffect: Send {
    fn process(&mut self, samples: &mut [f32], channels: usize, sample_rate: u32);
    fn reset(&mut self);
}

pub struct EffectChain {
    effects: Vec<Box<dyn AudioEffect>>,
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectChain {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    pub fn with(mut self, effect: impl AudioEffect + 'static) -> Self {
        self.effects.push(Box::new(effect));
        self
    }

    pub(crate) fn process(&mut self, samples: &mut [f32], channels: usize, sample_rate: u32) {
        for effect in &mut self.effects {
            effect.process(samples, channels, sample_rate);
        }
    }

    pub(crate) fn reset(&mut self) {
        for effect in &mut self.effects {
            effect.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ScaleEffect(f32);

    impl AudioEffect for ScaleEffect {
        fn process(&mut self, samples: &mut [f32], _channels: usize, _sample_rate: u32) {
            for s in samples.iter_mut() {
                *s *= self.0;
            }
        }
        fn reset(&mut self) {}
    }

    #[test]
    fn empty_chain_passthrough() {
        let mut chain = EffectChain::new();
        let mut samples = [1.0f32, 0.5, -0.5, 0.0];
        let original = samples;
        chain.process(&mut samples, 1, 44100);
        assert_eq!(samples, original);
    }

    #[test]
    fn chain_order() {
        let mut chain = EffectChain::new()
            .with(ScaleEffect(0.5))
            .with(ScaleEffect(0.5));
        let mut samples = [1.0f32; 4];
        chain.process(&mut samples, 1, 44100);
        for s in &samples {
            assert!((*s - 0.25).abs() < f32::EPSILON);
        }
    }
}
