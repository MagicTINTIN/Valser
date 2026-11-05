use rodio::Source;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// Center frequencies, 2/3 octave apart.
pub const BANDS: [f32; 16] = [
    20.0, 31.5, 50.0, 80.0, 125.0, 200.0, 315.0, 500.0, 800.0, 1250.0, 2000.0, 3150.0, 5000.0,
    8000.0, 12500.0, 20000.0,
];
pub const MAX_GAIN: f32 = 12.0;
/// Bandwidth of 2/3 octave.
const Q: f32 = 2.15;
/// Settings are checked every this many samples.
const REFRESH_EVERY: usize = 1024;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct EqConfig {
    pub enabled: bool,
    pub gains: [f32; 16],
}

impl Default for EqConfig {
    fn default() -> Self {
        Self { enabled: true, gains: [0.0; 16] }
    }
}

/// Shared between the player and the audio thread.
#[derive(Default)]
pub struct EqSettings {
    enabled: AtomicBool,
    gains: [AtomicU32; 16],
    version: AtomicU32,
}

impl EqSettings {
    pub fn set(&self, config: &EqConfig) {
        self.enabled.store(config.enabled, Ordering::Relaxed);
        for (gain, value) in self.gains.iter().zip(config.gains) {
            gain.store(value.clamp(-MAX_GAIN, MAX_GAIN).to_bits(), Ordering::Relaxed);
        }
        self.version.fetch_add(1, Ordering::Release);
    }

    fn get(&self) -> EqConfig {
        EqConfig {
            enabled: self.enabled.load(Ordering::Relaxed),
            gains: std::array::from_fn(|i| f32::from_bits(self.gains[i].load(Ordering::Relaxed))),
        }
    }
}

/// Peaking filter from the RBJ audio EQ cookbook, normalized by a0.
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

impl Biquad {
    fn peaking(freq: f32, gain_db: f32, sample_rate: f32) -> Self {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq.min(sample_rate * 0.45) / sample_rate;
        let alpha = w0.sin() / (2.0 * Q);
        let cos = w0.cos();
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * cos / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }

    /// Transposed direct form II, `z` is the filter memory of one channel.
    fn run(&self, x: f32, z: &mut [f32; 2]) -> f32 {
        let y = self.b0 * x + z[0];
        z[0] = self.b1 * x - self.a1 * y + z[1];
        z[1] = self.b2 * x - self.a2 * y;
        y
    }
}

pub struct Equalizer<S: Source<Item = f32>> {
    inner: S,
    settings: Arc<EqSettings>,
    version: u32,
    enabled: bool,
    preamp: f32,
    /// Only the bands with a non zero gain, with their index.
    filters: Vec<(usize, Biquad)>,
    /// Filter memory per channel and band.
    memory: Vec<[[f32; 2]; 16]>,
    channel: usize,
    counter: usize,
}

impl<S: Source<Item = f32>> Equalizer<S> {
    pub fn new(inner: S, settings: Arc<EqSettings>) -> Self {
        let channels = inner.channels().get() as usize;
        let mut eq = Self {
            inner,
            settings,
            version: 0,
            enabled: false,
            preamp: 1.0,
            filters: Vec::new(),
            memory: vec![[[0.0; 2]; 16]; channels],
            channel: 0,
            counter: 0,
        };
        eq.refresh();
        eq
    }

    fn refresh(&mut self) {
        self.version = self.settings.version.load(Ordering::Acquire);
        let config = self.settings.get();
        let rate = self.inner.sample_rate().get() as f32;
        let max_boost = config.gains.iter().fold(0f32, |max, &g| max.max(g));

        self.enabled = config.enabled;
        self.preamp = 10f32.powf(-max_boost / 20.0);
        self.filters = (0..16)
            .filter(|&i| config.gains[i] != 0.0)
            .map(|i| (i, Biquad::peaking(BANDS[i], config.gains[i], rate)))
            .collect();
    }
}

impl<S: Source<Item = f32>> Iterator for Equalizer<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.inner.next()?;
        let channel = self.channel;
        self.channel = (channel + 1) % self.memory.len();

        self.counter += 1;
        if self.counter >= REFRESH_EVERY && channel == 0 {
            self.counter = 0;
            if self.settings.version.load(Ordering::Acquire) != self.version {
                self.refresh();
            }
        }
        if !self.enabled {
            return Some(sample);
        }

        let memory = &mut self.memory[channel];
        let out = self
            .filters
            .iter()
            .fold(sample * self.preamp, |s, (band, filter)| filter.run(s, &mut memory[*band]));
        Some(out.clamp(-1.0, 1.0))
    }
}

impl<S: Source<Item = f32>> Source for Equalizer<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }

    fn channels(&self) -> rodio::ChannelCount {
        self.inner.channels()
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.memory.iter_mut().for_each(|m| *m = [[0.0; 2]; 16]);
        self.channel = 0;
        self.inner.try_seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;
    use std::num::NonZero;

    fn sine(freq: f32, rate: u32) -> Vec<f32> {
        (0..rate).map(|i| 0.5 * (2.0 * PI * freq * i as f32 / rate as f32).sin()).collect()
    }

    fn peak(samples: impl Iterator<Item = f32>) -> f32 {
        samples.skip(4000).fold(0.0, |max, s| max.max(s.abs()))
    }

    fn equalize(freq: f32, config: &EqConfig) -> f32 {
        let settings = Arc::new(EqSettings::default());
        settings.set(config);
        let source = SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(44100).unwrap(), sine(freq, 44100));
        peak(Equalizer::new(source, settings))
    }

    #[test]
    fn flat_is_unchanged() {
        assert!((equalize(1000.0, &EqConfig::default()) - 0.5).abs() < 0.01);
    }

    #[test]
    fn cut_lowers_its_band_only() {
        let mut config = EqConfig::default();
        config.gains[8] = -12.0;
        assert!(equalize(800.0, &config) < 0.15);
        assert!((equalize(8000.0, &config) - 0.5).abs() < 0.05);
    }

    #[test]
    fn boost_is_compensated_by_preamp() {
        let mut config = EqConfig::default();
        config.gains[8] = 12.0;
        assert!((equalize(800.0, &config) - 0.5).abs() < 0.05);
        assert!(equalize(8000.0, &config) < 0.15);
    }
}
