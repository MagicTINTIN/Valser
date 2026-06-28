use bevy::prelude::*;
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterScope {
    TrackName, // tag title, falls back to filename
    Artist,
    FileName, // raw filename, ignoring tags
}

impl Default for FilterScope {
    fn default() -> Self {
        FilterScope::TrackName
    }
}

/// Represents a single track in the playlist.
#[derive(Debug, Clone)]
pub struct Track {
    pub path: PathBuf,
    pub name: String,          // filename without extension (fallback display)
    pub title: Option<String>, // from tags
    pub artist: Option<String>,
    pub genres: Vec<String>, // split on ','
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

        let mut title = None;
        let mut artist = None;
        let mut genres = Vec::new();

        if let Ok(tagged_file) = lofty::read_from_path(&path) {
            use lofty::file::TaggedFileExt;
            use lofty::tag::Accessor;

            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                title = tag.title().map(|s| s.to_string());
                artist = tag.artist().map(|s| s.to_string());
                genres = tag.get_strings(lofty::tag::ItemKey::Genre).map(|g| g.trim().to_string()).collect();
            }
        }

        Self {
            path,
            name,
            title,
            artist,
            genres,
            duration: None,
        }
    }

    /// What to show in the playlist row: tag title if present, else filename.
    pub fn display_name(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    pub fn format_duration(d: Duration) -> String {
        let total = d.as_secs();
        let minutes = total / 60;
        let seconds = total % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    pub fn matches_filter(&self, query: &str, scope: FilterScope) -> bool {
        if query.is_empty() {
            return true;
        }
        let q = query.to_lowercase();
        match scope {
            FilterScope::TrackName => self.display_name().to_lowercase().contains(&q),
            FilterScope::Artist => self
                .artist
                .as_deref()
                .map(|a| a.to_lowercase().contains(&q))
                .unwrap_or(false),
            FilterScope::FileName => self.name.to_lowercase().contains(&q),
        }
    }
}

/// The global playlist and playback state.
#[derive(Resource, Default)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    /// Index of the currently playing/selected track.
    pub current: Option<usize>,
    /// If non-empty, ONLY tracks containing at least one of these genres are shown.
    pub genre_whitelist: HashSet<String>,
    /// Tracks containing any of these genres are hidden, even if whitelisted.
    pub genre_blacklist: HashSet<String>,
}

impl Playlist {
    pub fn add_tracks(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if is_supported_format(&path) {
                self.tracks.push(Track::new(path));
            }
        }
    }

    /// Recursively scans a directory and adds every supported audio file found.
    pub fn add_directory_recursive(&mut self, dir: &std::path::Path) {
        let paths: Vec<PathBuf> = walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| is_supported_format(path))
            .collect();
        self.add_tracks(paths);
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

    /// Shuffles the playlist, keeping current pointing at the same track.
    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let mut rng = rand::rng();

        // Remember which track is currently playing. (using path)
        let current_path = self.current.map(|i| self.tracks[i].path.clone());

        self.tracks.shuffle(&mut rng);

        // Re-find its new index after the shuffle.
        self.current = current_path.and_then(|p| self.tracks.iter().position(|t| t.path == p));
    }

    /// Returns true if this track should be visible given the current genre whitelist/blacklist settings.
    pub fn genre_visible(&self, track: &Track) -> bool {
        // Blacklist wins over whitelist.
        if track
            .genres
            .iter()
            .any(|g| self.genre_blacklist.contains(g))
        {
            return false;
        }
        if !self.genre_whitelist.is_empty()
            && !track
                .genres
                .iter()
                .any(|g| self.genre_whitelist.contains(g))
        {
            return false;
        }
        true
    }

    /// Sorted map of genre -> count of tracks currently in the playlist
    /// having that genre (counts ALL tracks, regardless of current
    /// whitelist/blacklist, so the sidebar can show "what exists").
    pub fn genre_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for track in &self.tracks {
            for genre in &track.genres {
                *counts.entry(genre.clone()).or_insert(0) += 1;
            }
        }
        counts
    }

    pub fn toggle_whitelist(&mut self, genre: &str) {
        if self.genre_whitelist.contains(genre) {
            self.genre_whitelist.remove(genre);
        } else {
            self.genre_whitelist.insert(genre.to_string());
            self.genre_blacklist.remove(genre); // mutually exclusive
        }
    }

    pub fn toggle_blacklist(&mut self, genre: &str) {
        if self.genre_blacklist.contains(genre) {
            self.genre_blacklist.remove(genre);
        } else {
            self.genre_blacklist.insert(genre.to_string());
            self.genre_whitelist.remove(genre); // mutually exclusive
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
        Some(
            "mp3" | "ogg" | "opus" | "flac" | "wav" | "m4a" | "aac" | "aiff" | "alac" | "mp4"
            | "webm" | "mkv",
        ) => true,
        _ => false,
    }
}

pub struct PlaylistPlugin;

impl Plugin for PlaylistPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playlist>();
    }
}
