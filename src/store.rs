use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

// Table definitions
/// track_id -> JSON-serialized TrackRecord
const TRACKS: TableDefinition<u64, &str> = TableDefinition::new("tracks");
/// "next_id" -> next available track ID
const META: TableDefinition<&str, u64> = TableDefinition::new("meta");
/// "track_id" -> track ID currently playing; "position_secs" -> f64 as bits
const PLAYBACK: TableDefinition<&str, u64> = TableDefinition::new("playback");

/// stored in library.redb
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackRecord {
    pub id: u64,
    pub path: String,
    pub name: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub genres: Vec<String>,
    pub duration_secs: Option<f64>,
}

/// Stored in state.json
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppState {
    /// Ordered list of track IDs — this IS the playlist order after shuffles.
    pub playlist_order: Vec<u64>,
    pub current_index: Option<usize>,
    pub filter_text: String,
    pub filter_scope: String, // "name" | "artist" | "filename"
    pub genre_whitelist: Vec<String>,
    pub genre_blacklist: Vec<String>,
}

pub struct Store {
    library: Database, // all the tracks added in the player
    playback: Database, // all tracks passing filters
    state_path: PathBuf, // player state file path
    settings_path: PathBuf, // app settings file path
}

impl Store {
    pub fn open(data_dir: &Path, config_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(data_dir)?;
        std::fs::create_dir_all(config_dir)?;
        let library = Database::create(data_dir.join("library.redb"))?;
        let playback = Database::create(data_dir.join("playback.redb"))?;

        // create tables if they don't exist
        {
            let tx = library.begin_write()?;
            tx.open_table(TRACKS)?;
            tx.open_table(META)?;
            tx.commit()?;
        }
        {
            let tx = playback.begin_write()?;
            tx.open_table(PLAYBACK)?;
            tx.commit()?;
        }

        Ok(Self {
            library,
            playback,
            state_path: data_dir.join("state.json"),
            settings_path: config_dir.join("settings.json"),
        })
    }
}
