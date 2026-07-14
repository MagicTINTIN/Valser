use rodio::Source;
use std::fs::File;
use std::path::Path;
use std::time::Duration;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{CodecRegistry, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia_adapter_libopus::OpusDecoder;

pub struct OpusSource {
    samples: Vec<f32>,
    pos: usize,
    sample_rate: u32,
    channels: u16,
    total_duration: Option<Duration>,
}

impl OpusSource {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let mut codec_registry = CodecRegistry::new();
        codec_registry.register_all::<OpusDecoder>();

        // In Symphonia 0.5, File implements MediaSource directly — no BufReader wrapper
        let file = Box::new(File::open(path)?);
        let mss = MediaSourceStream::new(file, Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        // In Symphonia 0.5, Probe is at symphonia::core::probe
        let mut probe = symphonia::core::probe::Probe::default();
        symphonia::default::register_enabled_formats(&mut probe);

        let probed = probe.format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let mut format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or("No audio track found")?
            .clone();

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(48_000);
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);

        let mut decoder = codec_registry.make(&track.codec_params, &DecoderOptions::default())?;

        let mut all_samples: Vec<f32> = Vec::new();

        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(_)) => break,
                Err(symphonia::core::errors::Error::ResetRequired) => break,
                Err(e) => return Err(e.into()),
            };

            if packet.track_id() != track_id {
                continue;
            }

            match decoder.decode(&packet)? {
                AudioBufferRef::F32(buf) => {
                    for i in 0..buf.frames() {
                        for ch in 0..channels as usize {
                            all_samples.push(buf.chan(ch)[i]);
                        }
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for i in 0..buf.frames() {
                        for ch in 0..channels as usize {
                            all_samples.push(buf.chan(ch)[i] as f32 / 32768.0);
                        }
                    }
                }
                AudioBufferRef::S32(buf) => {
                    for i in 0..buf.frames() {
                        for ch in 0..channels as usize {
                            all_samples.push(buf.chan(ch)[i] as f32 / i32::MAX as f32);
                        }
                    }
                }
                _ => {} // other formats unlikely for opus but safe to skip
            }
        }

        let total_samples = all_samples.len() / channels as usize;
        let total_duration = Some(Duration::from_secs_f64(
            total_samples as f64 / sample_rate as f64,
        ));

        Ok(Self { samples: all_samples, pos: 0, sample_rate, channels, total_duration })
    }
}

// Iterator and Source impls are unchanged from your original
impl Iterator for OpusSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let s = self.samples.get(self.pos).copied();
        if s.is_some() { self.pos += 1; }
        s
    }
}

impl Source for OpusSource {
    fn current_span_len(&self) -> Option<usize> { None }
    fn channels(&self) -> std::num::NonZero<u16> {
        std::num::NonZero::new(self.channels).unwrap()
    }
    fn sample_rate(&self) -> std::num::NonZero<u32> {
        std::num::NonZero::new(self.sample_rate).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> { self.total_duration }
    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        let target = (pos.as_secs_f64() * self.sample_rate as f64) as usize * self.channels as usize;
        self.pos = target.min(self.samples.len());
        Ok(())
    }
}