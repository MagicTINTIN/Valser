use bevy::log::error;
use bevy::prelude::*;
use rodio::source::Source;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Duration;



use crate::opus_source::OpusSource;

// Resources

#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

#[derive(Resource, Default)]
pub struct AudioCommand {
    pub play: Option<std::path::PathBuf>,
    pub toggle_pause: bool,
    pub stop: bool,
    pub seek: Option<Duration>,
    pub volume: Option<f32>,
}

#[derive(Resource)]
pub struct PlaybackInfo {
    pub position: Duration,
    pub duration: Option<Duration>,
    pub volume: f32,
}

impl Default for PlaybackInfo {
    fn default() -> Self {
        Self { position: Duration::ZERO, duration: None, volume: 1.0 }
    }
}

// Non-Send audio state (CPAL streams are not Send)

pub struct AudioState {
    /// Keeps the MixerDeviceSink alive, playback stops if dropped.
    _handle: MixerDeviceSink,
    player: Player,
    pub duration: Option<Duration>,
}

impl AudioState {
    fn new() -> Option<Self> {
        let handle = DeviceSinkBuilder::open_default_sink().ok()?;
        let player = Player::connect_new(&handle.mixer());
        player.set_volume(1.0);
        Some(Self { _handle: handle, player, duration: None })
    }

    fn load_and_play(&mut self, path: &Path) -> Option<Duration> {
    self.player.stop();

    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "opus" {
        match OpusSource::new(path) {
            Ok(source) => {
                let duration = source.total_duration();
                self.duration = duration;
                self.player.append(source);
                self.player.play();
                duration
            }
            Err(e) => {
                bevy::log::error!("Failed to decode opus file: {e}");
                None
            }
        }
    } else {
        let file = File::open(path).ok()?;
        let decoder = Decoder::try_from(BufReader::new(file)).ok()?;
        let duration = decoder.total_duration();
        self.duration = duration;
        self.player.append(decoder);
        self.player.play();
        duration
    }
}

    fn toggle_pause(&mut self) {
        if self.player.is_paused() { self.player.play(); } else { self.player.pause(); }
    }

    fn stop(&mut self) {
        self.player.stop();
        self.duration = None;
    }

    fn seek(&mut self, pos: Duration) { let _ = self.player.try_seek(pos); }
    fn set_volume(&mut self, v: f32) { self.player.set_volume(v); }
    fn is_paused(&self) -> bool { self.player.is_paused() }
    fn is_empty(&self) -> bool { self.player.empty() }
    fn position(&self) -> Duration { self.player.get_pos() }
}

// Messages

#[derive(Message, Clone)]
pub struct TrackFinished;

// Systems

fn setup_audio(world: &mut World) {
    match AudioState::new() {
        Some(state) => world.insert_non_send_resource(state),
        None => error!("Failed to open audio output device"),
    }
}

fn update_audio(
    mut commands_res: ResMut<AudioCommand>,
    mut playback_state: ResMut<PlaybackState>,
    mut playback_info: ResMut<PlaybackInfo>,
    mut track_finished: MessageWriter<TrackFinished>,
    audio: Option<NonSendMut<AudioState>>,
) {
    let Some(mut audio) = audio else { return };

    if let Some(path) = commands_res.play.take() {
        let duration = audio.load_and_play(&path);
        playback_info.duration = duration;
        playback_info.position = Duration::ZERO;
        *playback_state = PlaybackState::Playing;
    }

    if commands_res.toggle_pause {
        commands_res.toggle_pause = false;
        audio.toggle_pause();
        *playback_state = if audio.is_paused() { PlaybackState::Paused } else { PlaybackState::Playing };
    }

    if commands_res.stop {
        commands_res.stop = false;
        audio.stop();
        playback_info.duration = None;
        playback_info.position = Duration::ZERO;
        *playback_state = PlaybackState::Stopped;
    }

    if let Some(pos) = commands_res.seek.take() { audio.seek(pos); }
    if let Some(vol) = commands_res.volume.take() {
        audio.set_volume(vol);
        playback_info.volume = vol;
    }

    if *playback_state == PlaybackState::Playing {
        playback_info.position = audio.position();
        if audio.is_empty() {
            *playback_state = PlaybackState::Stopped;
            playback_info.position = Duration::ZERO;
            track_finished.write(TrackFinished);
        }
    }
}

// --------------------------------------------------------------------------
// Plugin

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioCommand>()
            .init_resource::<PlaybackState>()
            .init_resource::<PlaybackInfo>()
            .add_message::<TrackFinished>()
            .add_systems(Startup, setup_audio)
            .add_systems(Update, update_audio);
    }
}
