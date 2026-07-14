use rodio::Source;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;

pub struct OpusSource {
    samples: Vec<f32>,
    pos: usize,           // current read position (sample index)
    sample_rate: u32,
    channels: u16,
    total_duration: Option<Duration>,
}

impl OpusSource {
    pub fn new(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let mut ogg = ogg::PacketReader::new(BufReader::new(file));

        // Read the OpusHead packet to get channel count and pre-skip.
        let head_packet = ogg.read_packet_expected()?;
        if &head_packet.data[0..8] != b"OpusHead" {
            return Err("Not an Opus stream".into());
        }
        let channels = head_packet.data[9] as u16;
        let pre_skip = u16::from_le_bytes([head_packet.data[10], head_packet.data[11]]) as usize;
        // input_sample_rate is at bytes 12..16 but Opus always outputs at 48000 Hz.
        let sample_rate: u32 = 48_000;

        // Skip the OpusTags comment packet.
        ogg.read_packet_expected()?;

        // Decode all audio packets up front.
        let mut decoder = opus::Decoder::new(sample_rate, if channels == 2 {
            opus::Channels::Stereo
        } else {
            opus::Channels::Mono
        })?;

        let mut all_samples: Vec<f32> = Vec::new();
        let max_frame = 5760 * channels as usize; // 120 ms at 48 kHz
        let mut pcm_buf = vec![0i16; max_frame];

        while let Ok(Some(packet)) = ogg.read_packet() {
            let n = decoder.decode(&packet.data, &mut pcm_buf, false)?;
            for &s in &pcm_buf[..n * channels as usize] {
                all_samples.push(s as f32 / 32768.0);
            }
        }

        // Remove the pre-skip samples.
        let skip = pre_skip * channels as usize;
        let samples: Vec<f32> = all_samples.into_iter().skip(skip).collect();

        let total_samples = samples.len() / channels as usize;
        let total_duration = Some(Duration::from_secs_f64(
            total_samples as f64 / sample_rate as f64,
        ));

        Ok(Self { samples, pos: 0, sample_rate, channels, total_duration })
    }
}

impl Iterator for OpusSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let s = self.samples.get(self.pos).copied();
        if s.is_some() {
            self.pos += 1;
        }
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
        let target_sample = (pos.as_secs_f64() * self.sample_rate as f64) as usize
            * self.channels as usize;
        self.pos = target_sample.min(self.samples.len());
        Ok(())
    }
}