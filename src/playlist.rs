use bevy::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

/// Represents a single track in the playlist.
#[derive(Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    /// Display name derived from the filename.
    pub name: String,
    /// Total duration, populated once the track has been decoded once.
    pub duration: Option<Duration>,
}

impl Track {
    pub fn new(path: PathBuf) -> Self {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        Self {
            path,
            name,
            duration: None,
        }
    }

    /// Pretty-formats a duration as mm:ss.
    pub fn format_duration(d: Duration) -> String {
        let total = d.as_secs();
        let minutes = total / 60;
        let seconds = total % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// The global playlist and playback state.
#[derive(Resource, Default)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    /// Index of the currently playing/selected track.
    pub current: Option<usize>,
}

impl Playlist {
    pub fn add_tracks(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if is_supported_format(&path) {
                self.tracks.push(Track::new(path));
            }
        }
    }

    pub fn remove_track(&mut self, index: usize) {
        self.tracks.remove(index);
        // Adjust current index after removal.
        if let Some(cur) = self.current {
            if cur == index {
                self.current = None;
            } else if cur > index {
                self.current = Some(cur - 1);
            }
        }
    }

    pub fn next_track(&self) -> Option<usize> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }
        match self.current {
            None => Some(0),
            Some(i) if i + 1 < len => Some(i + 1),
            _ => None,
        }
    }

    pub fn prev_track(&self) -> Option<usize> {
        match self.current {
            Some(i) if i > 0 => Some(i - 1),
            _ => None,
        }
    }
}

/// Returns true if the file extension is one rodio/symphonia can decode.
fn is_supported_format(path: &std::path::Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("mp3" | "ogg" | "opus" | "flac" | "wav" | "m4a" | "aac" | "aiff" | "alac"
            | "mp4" | "webm" | "mkv") => true,
        _ => false,
    }
}

pub struct PlaylistPlugin;

impl Plugin for PlaylistPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playlist>();
    }
}
