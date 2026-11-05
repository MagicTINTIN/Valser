use rodio::cpal::StreamError;
use rodio::source::Source;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use serde::Serialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::equalizer::{EqSettings, Equalizer};
use crate::opus_source::OpusSource;

#[derive(Default, PartialEq, Clone, Copy, Debug, Serialize)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Not Send (cpal streams), so it must live on the thread that created it.
pub struct Audio {
    _sink: MixerDeviceSink,
    player: Player,
    eq: Arc<EqSettings>,
}

impl Audio {
    /// `on_lost` is called from the audio thread when the output device goes away,
    /// for example when headphones are plugged or unplugged.
    pub fn new(eq: Arc<EqSettings>, on_lost: impl Fn() + Send + 'static) -> Option<Self> {
        let on_error = move |err: StreamError| {
            if matches!(err, StreamError::DeviceNotAvailable | StreamError::StreamInvalidated) {
                on_lost();
            }
        };
        let sink = DeviceSinkBuilder::from_default_device()
            .and_then(|builder| builder.with_error_callback(on_error).open_stream())
            .or_else(|_| DeviceSinkBuilder::open_default_sink())
            .ok()?;
        let player = Player::connect_new(sink.mixer());
        Some(Self { _sink: sink, player, eq })
    }

    /// Starts playing the file and returns its duration if known.
    pub fn play(&mut self, path: &Path) -> Option<Duration> {
        self.player.stop();
        let is_opus = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("opus"));

        let duration = if is_opus {
            let source = OpusSource::new(path)
                .map_err(|e| eprintln!("Failed to decode opus file: {e}"))
                .ok()?;
            let duration = source.total_duration();
            self.player.append(Equalizer::new(source, self.eq.clone()));
            duration
        } else {
            let decoder = Decoder::try_from(BufReader::new(File::open(path).ok()?)).ok()?;
            let duration = decoder.total_duration();
            self.player.append(Equalizer::new(decoder, self.eq.clone()));
            duration
        };
        self.player.play();
        duration
    }

    pub fn toggle_pause(&mut self) {
        if self.player.is_paused() {
            self.player.play();
        } else {
            self.player.pause();
        }
    }

    pub fn stop(&mut self) {
        self.player.stop();
    }

    pub fn seek(&mut self, pos: Duration) {
        let _ = self.player.try_seek(pos);
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.player.set_volume(volume);
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }

    pub fn position(&self) -> Duration {
        self.player.get_pos()
    }
}
