use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::store::TrackRecord;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum FilterScope {
    #[default]
    #[serde(rename = "name")]
    TrackName,
    #[serde(rename = "artist")]
    Artist,
    #[serde(rename = "filename")]
    FileName,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub genres: Vec<String>,
    pub duration: Option<Duration>,
}

impl Track {
    /// Reads tags from the file, the id is set when inserted in the store.
    pub fn new(path: PathBuf) -> Self {
        use lofty::file::TaggedFileExt;
        use lofty::tag::{Accessor, ItemKey};

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        let tagged = lofty::read_from_path(&path).ok();
        let tag = tagged
            .as_ref()
            .and_then(|f| f.primary_tag().or_else(|| f.first_tag()));

        Self {
            id: 0,
            name,
            title: tag.and_then(|t| t.title()).map(|s| s.to_string()),
            artist: tag.and_then(|t| t.artist()).map(|s| s.to_string()),
            genres: tag
                .map(|t| t.get_strings(ItemKey::Genre).map(|g| g.trim().to_string()).collect())
                .unwrap_or_default(),
            duration: None,
            path,
        }
    }

    pub fn display_name(&self) -> &str {
        &self.name
    }

    pub fn matches_filter(&self, query: &str, scope: FilterScope) -> bool {
        let q = query.to_lowercase();
        match scope {
            FilterScope::TrackName => self
                .title
                .as_deref()
                .is_some_and(|t| t.to_lowercase().contains(&q)),
            FilterScope::Artist => self
                .artist
                .as_deref()
                .is_some_and(|a| a.to_lowercase().contains(&q)),
            FilterScope::FileName => self.name.to_lowercase().contains(&q),
        }
    }

    pub fn to_record(&self) -> TrackRecord {
        TrackRecord {
            id: self.id,
            path: self.path.to_string_lossy().to_string(),
            name: self.name.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            genres: self.genres.clone(),
            duration_secs: self.duration.map(|d| d.as_secs_f64()),
        }
    }

    pub fn from_record(record: TrackRecord) -> Self {
        Self {
            id: record.id,
            path: PathBuf::from(&record.path),
            name: record.name,
            title: record.title,
            artist: record.artist,
            genres: record.genres,
            duration: record.duration_secs.map(Duration::from_secs_f64),
        }
    }
}

#[derive(Default)]
pub struct Playlist {
    pub tracks: Vec<Track>,
    pub current: Option<usize>,
    /// If non empty, only tracks having one of these genres are shown.
    pub genre_whitelist: HashSet<String>,
    /// Tracks having one of these genres are hidden, even if whitelisted.
    pub genre_blacklist: HashSet<String>,
}

impl Playlist {
    pub fn remove_track(&mut self, index: usize) {
        self.tracks.remove(index);
        self.current = match self.current {
            Some(cur) if cur == index => None,
            Some(cur) if cur > index => Some(cur - 1),
            cur => cur,
        };
    }

    pub fn next_track(&self) -> Option<usize> {
        match self.current {
            None if !self.tracks.is_empty() => Some(0),
            Some(i) if i + 1 < self.tracks.len() => Some(i + 1),
            _ => None,
        }
    }

    pub fn prev_track(&self) -> Option<usize> {
        self.current.filter(|&i| i > 0).map(|i| i - 1)
    }

    /// Shuffles the tracks, current keeps pointing to the same track.
    pub fn shuffle(&mut self) {
        use rand::seq::SliceRandom;
        let current_id = self.current.map(|i| self.tracks[i].id);
        self.tracks.shuffle(&mut rand::rng());
        self.current = current_id.and_then(|id| self.tracks.iter().position(|t| t.id == id));
    }

    pub fn genre_visible(&self, track: &Track) -> bool {
        let has = |set: &HashSet<String>| track.genres.iter().any(|g| set.contains(g));
        !has(&self.genre_blacklist) && (self.genre_whitelist.is_empty() || has(&self.genre_whitelist))
    }

    pub fn genre_counts(&self) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for genre in self.tracks.iter().flat_map(|t| &t.genres) {
            *counts.entry(genre.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub fn toggle_whitelist(&mut self, genre: &str) {
        if !self.genre_whitelist.remove(genre) {
            self.genre_whitelist.insert(genre.to_string());
            self.genre_blacklist.remove(genre);
        }
    }

    pub fn toggle_blacklist(&mut self, genre: &str) {
        if !self.genre_blacklist.remove(genre) {
            self.genre_blacklist.insert(genre.to_string());
            self.genre_whitelist.remove(genre);
        }
    }
}

pub fn is_supported_format(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase());
    matches!(
        ext.as_deref(),
        Some(
            "mp3" | "ogg" | "opus" | "flac" | "wav" | "m4a" | "aac" | "aiff" | "alac" | "mp4"
                | "webm" | "mkv"
        )
    )
}

/// Expands directories recursively and keeps only supported audio files.
pub fn collect_audio_files(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .flat_map(walkdir::WalkDir::new)
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| is_supported_format(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: u64, genres: &[&str]) -> Track {
        Track {
            id,
            path: PathBuf::from(format!("{id}.mp3")),
            name: id.to_string(),
            title: None,
            artist: None,
            genres: genres.iter().map(|g| g.to_string()).collect(),
            duration: None,
        }
    }

    #[test]
    fn navigation_and_removal() {
        let mut playlist = Playlist {
            tracks: vec![track(1, &[]), track(2, &[]), track(3, &[])],
            ..Default::default()
        };
        assert_eq!(playlist.next_track(), Some(0));
        playlist.current = Some(2);
        assert_eq!(playlist.next_track(), None);
        assert_eq!(playlist.prev_track(), Some(1));
        playlist.remove_track(0);
        assert_eq!(playlist.current, Some(1));
        playlist.remove_track(1);
        assert_eq!(playlist.current, None);
    }

    #[test]
    fn genre_filters() {
        let mut playlist = Playlist::default();
        let rock = track(1, &["rock"]);
        let jazz = track(2, &["jazz"]);
        playlist.toggle_whitelist("rock");
        assert!(playlist.genre_visible(&rock));
        assert!(!playlist.genre_visible(&jazz));
        playlist.toggle_blacklist("rock");
        assert!(playlist.genre_whitelist.is_empty());
        assert!(!playlist.genre_visible(&rock));
        assert!(playlist.genre_visible(&jazz));
    }

    #[test]
    fn old_filter_scope_strings() {
        let scope: FilterScope = serde_json::from_str("\"artist\"").unwrap();
        assert_eq!(scope, FilterScope::Artist);
    }
}
