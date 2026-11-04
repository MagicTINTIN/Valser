use crossbeam_channel::{RecvTimeoutError, Sender};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};
use valser_core::audio::{Audio, PlaybackState};
use valser_core::playlist::{FilterScope, Playlist, Track, collect_audio_files};
use valser_core::store::{AppState, Store, TrackRecord};

const TICK: Duration = Duration::from_millis(250);
const SAVE_POSITION_EVERY: Duration = Duration::from_secs(5);
const LOAD_CHUNK: usize = 100;

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cmd {
    Ready,
    Play { index: usize },
    Toggle,
    Pause,
    Resume,
    Stop,
    Next,
    Prev,
    Seek { secs: f64 },
    SeekBy { secs: f64 },
    Volume { value: f32 },
    Remove { index: usize },
    Clear,
    Shuffle,
    Filter { text: String, scope: FilterScope },
    Genre { name: String, blacklist: bool },
    ClearGenres,
    Add { paths: Vec<PathBuf> },
    #[serde(skip)]
    Queued(usize),
    #[serde(skip)]
    Loaded(Vec<TrackRecord>),
}

/// Playback info shared with the system media controls.
pub struct MediaState {
    pub state: PlaybackState,
    pub title: String,
    pub artist: String,
    pub length: f64,
    pub position: f64,
}

#[derive(Serialize)]
struct Row<'a> {
    index: usize,
    name: &'a str,
    duration: Option<f64>,
}

#[derive(Serialize)]
struct Genre {
    name: String,
    count: usize,
    mode: &'static str,
}

#[derive(Serialize)]
struct PlaylistView<'a> {
    rows: Vec<Row<'a>>,
    total: usize,
    genres: Vec<Genre>,
    filter: &'a str,
    scope: FilterScope,
}

#[derive(Serialize)]
struct Status<'a> {
    state: PlaybackState,
    position: f64,
    duration: Option<f64>,
    volume: f32,
    current: Option<usize>,
    title: Option<&'a str>,
    has_prev: bool,
    has_next: bool,
    loaded: usize,
    queued: usize,
}

struct Engine {
    app: AppHandle,
    tx: Sender<Cmd>,
    store: Store,
    audio: Option<Audio>,
    playlist: Playlist,
    filter: String,
    scope: FilterScope,
    state: PlaybackState,
    duration: Option<Duration>,
    volume: f32,
    queued: usize,
    loaded: usize,
    resume: Option<(u64, f64)>,
    last_save: Instant,
    #[cfg(target_os = "linux")]
    mpris: smol::channel::Sender<MediaState>,
}

pub fn spawn(app: AppHandle, store: Store) -> Sender<Cmd> {
    let (tx, rx) = crossbeam_channel::unbounded();
    #[cfg(target_os = "linux")]
    let mpris = crate::mpris::spawn(tx.clone());
    #[cfg(target_os = "android")]
    crate::android::listen(tx.clone());
    let engine_tx = tx.clone();

    std::thread::spawn(move || {
        let audio = Audio::new();
        if audio.is_none() {
            eprintln!("Failed to open audio output device");
        }
        let mut engine = Engine {
            app,
            tx: engine_tx,
            store,
            audio,
            playlist: Playlist::default(),
            filter: String::new(),
            scope: FilterScope::default(),
            state: PlaybackState::Stopped,
            duration: None,
            volume: 1.0,
            queued: 0,
            loaded: 0,
            resume: None,
            last_save: Instant::now(),
            #[cfg(target_os = "linux")]
            mpris,
        };
        engine.restore();
        loop {
            match rx.recv_timeout(TICK) {
                Ok(cmd) => engine.handle(cmd),
                Err(RecvTimeoutError::Timeout) => engine.tick(),
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    tx
}

impl Engine {
    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Ready => self.emit_playlist(),
            Cmd::Play { index } => self.play(index),
            Cmd::Toggle => self.toggle(),
            Cmd::Pause if self.state == PlaybackState::Playing => self.toggle(),
            Cmd::Resume if self.state != PlaybackState::Playing => self.toggle(),
            Cmd::Pause | Cmd::Resume => {}
            Cmd::Stop => self.stop(),
            Cmd::Next => self.playlist.next_track().into_iter().for_each(|i| self.play(i)),
            Cmd::Prev => self.playlist.prev_track().into_iter().for_each(|i| self.play(i)),
            Cmd::Seek { secs } => self.seek(secs),
            Cmd::SeekBy { secs } => {
                let position = self.audio.as_ref().map_or(0.0, |a| a.position().as_secs_f64());
                self.seek(position + secs);
            }
            Cmd::Volume { value } => {
                self.volume = value;
                if let Some(audio) = &mut self.audio {
                    audio.set_volume(value);
                }
            }
            Cmd::Remove { index } => {
                let Some(track) = self.playlist.tracks.get(index) else { return };
                let _ = self.store.remove_track(track.id);
                let was_current = self.playlist.current == Some(index);
                self.playlist.remove_track(index);
                if was_current {
                    self.stop();
                }
                self.changed();
            }
            Cmd::Clear => {
                let _ = self.store.clear_all_tracks();
                self.playlist.tracks.clear();
                self.playlist.current = None;
                self.stop();
                self.changed();
            }
            Cmd::Shuffle => {
                self.playlist.shuffle();
                self.changed();
            }
            Cmd::Filter { text, scope } => {
                self.filter = text;
                self.scope = scope;
                self.changed();
            }
            Cmd::Genre { name, blacklist: true } => {
                self.playlist.toggle_blacklist(&name);
                self.changed();
            }
            Cmd::Genre { name, blacklist: false } => {
                self.playlist.toggle_whitelist(&name);
                self.changed();
            }
            Cmd::ClearGenres => {
                self.playlist.genre_whitelist.clear();
                self.playlist.genre_blacklist.clear();
                self.changed();
            }
            Cmd::Add { paths } => self.load(paths),
            Cmd::Queued(count) => {
                if self.loaded == self.queued {
                    self.loaded = 0;
                    self.queued = 0;
                }
                self.queued += count;
            }
            Cmd::Loaded(records) => self.insert(records),
        }
        self.emit_status();
    }

    fn tick(&mut self) {
        if self.state != PlaybackState::Playing {
            return;
        }
        if self.audio.as_ref().is_some_and(|a| a.is_empty()) {
            match self.playlist.next_track() {
                Some(next) => self.play(next),
                None => self.stop(),
            }
        } else if self.last_save.elapsed() >= SAVE_POSITION_EVERY {
            self.save_position();
        }
        self.emit_status();
    }

    fn play(&mut self, index: usize) {
        let Some(track) = self.playlist.tracks.get(index) else { return };
        let Some(audio) = &mut self.audio else { return };
        self.playlist.current = Some(index);
        self.duration = audio.play(&track.path);
        if let Some((id, secs)) = self.resume.take()
            && id == track.id
        {
            audio.seek(Duration::from_secs_f64(secs));
        }
        self.state = PlaybackState::Playing;
        self.save_state();
    }

    fn toggle(&mut self) {
        if self.state == PlaybackState::Stopped {
            if let Some(index) = self.playlist.current.or_else(|| self.playlist.next_track()) {
                self.play(index);
            }
        } else if let Some(audio) = &mut self.audio {
            audio.toggle_pause();
            self.state = if audio.is_paused() { PlaybackState::Paused } else { PlaybackState::Playing };
        }
    }

    fn stop(&mut self) {
        if let Some(audio) = &mut self.audio {
            audio.stop();
        }
        self.state = PlaybackState::Stopped;
        self.duration = None;
    }

    fn seek(&mut self, secs: f64) {
        if let Some(audio) = &mut self.audio {
            audio.seek(Duration::from_secs_f64(secs.max(0.0)));
        }
    }

    /// Tags are read on a separate thread, results come back as commands.
    fn load(&self, paths: Vec<PathBuf>) {
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let files = collect_audio_files(paths);
            let _ = tx.send(Cmd::Queued(files.len()));
            for chunk in files.chunks(LOAD_CHUNK) {
                let records = chunk.iter().map(|p| Track::new(p.clone()).to_record()).collect();
                let _ = tx.send(Cmd::Loaded(records));
            }
        });
    }

    fn insert(&mut self, records: Vec<TrackRecord>) {
        self.loaded += records.len();
        match self.store.insert_tracks_batch(&records) {
            Ok(ids) => {
                for (record, id) in records.into_iter().zip(ids).filter_map(|(r, id)| Some((r, id?))) {
                    if !self.playlist.tracks.iter().any(|t| t.id == id) {
                        let mut track = Track::from_record(record);
                        track.id = id;
                        self.playlist.tracks.push(track);
                    }
                }
            }
            Err(e) => eprintln!("Batch insert failed: {e}"),
        }
        if self.playlist.current.is_none() && !self.playlist.tracks.is_empty() {
            self.playlist.current = Some(0);
        }
        if self.loaded >= self.queued {
            self.changed();
        }
    }

    fn restore(&mut self) {
        let state = self.store.load_state();
        let records = if state.playlist_order.is_empty() {
            self.store.load_all_tracks()
        } else {
            self.store.load_tracks_by_ids(&state.playlist_order)
        };
        self.playlist.tracks = records.unwrap_or_default().into_iter().map(Track::from_record).collect();
        self.playlist.genre_whitelist = state.genre_whitelist.into_iter().collect();
        self.playlist.genre_blacklist = state.genre_blacklist.into_iter().collect();
        self.playlist.current = state.current_index.filter(|&i| i < self.playlist.tracks.len());
        self.filter = state.filter_text;
        self.scope = state.filter_scope;

        if let Some((id, secs)) = self.store.load_position()
            && let Some(index) = self.playlist.tracks.iter().position(|t| t.id == id)
        {
            self.playlist.current = Some(index);
            self.resume = Some((id, secs));
        }
    }

    fn changed(&mut self) {
        self.save_state();
        self.emit_playlist();
    }

    fn save_state(&self) {
        let state = AppState {
            playlist_order: self.playlist.tracks.iter().map(|t| t.id).collect(),
            current_index: self.playlist.current,
            filter_text: self.filter.clone(),
            filter_scope: self.scope,
            genre_whitelist: self.playlist.genre_whitelist.iter().cloned().collect(),
            genre_blacklist: self.playlist.genre_blacklist.iter().cloned().collect(),
        };
        if let Err(e) = self.store.save_state(&state) {
            eprintln!("Failed to save state: {e}");
        }
    }

    fn save_position(&mut self) {
        self.last_save = Instant::now();
        if let (Some(track), Some(audio)) = (self.current_track(), &self.audio) {
            let _ = self.store.save_position(track.id, audio.position().as_secs_f64());
        }
    }

    fn current_track(&self) -> Option<&Track> {
        self.playlist.current.and_then(|i| self.playlist.tracks.get(i))
    }

    fn emit_playlist(&self) {
        let playlist = &self.playlist;
        let rows = playlist
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.matches_filter(&self.filter, self.scope) && playlist.genre_visible(t))
            .map(|(index, t)| Row {
                index,
                name: t.display_name(),
                duration: t.duration.map(|d| d.as_secs_f64()),
            })
            .collect();
        let genres = playlist
            .genre_counts()
            .into_iter()
            .map(|(name, count)| Genre {
                mode: if playlist.genre_whitelist.contains(&name) {
                    "whitelist"
                } else if playlist.genre_blacklist.contains(&name) {
                    "blacklist"
                } else {
                    ""
                },
                name,
                count,
            })
            .collect();
        let view = PlaylistView {
            rows,
            total: playlist.tracks.len(),
            genres,
            filter: &self.filter,
            scope: self.scope,
        };
        let _ = self.app.emit("playlist", &view);
    }

    fn emit_status(&self) {
        let position = match self.state {
            PlaybackState::Stopped => 0.0,
            _ => self.audio.as_ref().map_or(0.0, |a| a.position().as_secs_f64()),
        };
        let track = self.current_track();
        let status = Status {
            state: self.state,
            position,
            duration: self.duration.map(|d| d.as_secs_f64()),
            volume: self.volume,
            current: self.playlist.current,
            title: track.map(|t| t.display_name()),
            has_prev: self.playlist.prev_track().is_some(),
            has_next: self.playlist.next_track().is_some(),
            loaded: self.loaded,
            queued: self.queued,
        };
        let _ = self.app.emit("status", &status);

        #[cfg(any(target_os = "linux", target_os = "android"))]
        let media = MediaState {
            state: self.state,
            title: status.title.unwrap_or_default().to_string(),
            artist: track.and_then(|t| t.artist.clone()).unwrap_or_default(),
            length: status.duration.unwrap_or(0.0),
            position,
        };
        #[cfg(target_os = "android")]
        crate::android::update(&media);
        #[cfg(target_os = "linux")]
        let _ = self.mpris.try_send(media);
    }
}
