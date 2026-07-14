use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition, TableHandle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    /// Ordered list of track IDs, this is the playlist order after shuffles.
    pub playlist_order: Vec<u64>,
    pub current_index: Option<usize>,
    pub filter_text: String,
    pub filter_scope: String, // "name" | "artist" | "filename"
    pub genre_whitelist: Vec<String>,
    pub genre_blacklist: Vec<String>,
}

pub struct Store {
    library: Database,      // all the tracks added in the player
    playback: Database,     // all tracks passing filters
    state_path: PathBuf,    // player state file path
    _settings_path: PathBuf, // app settings file path
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
            _settings_path: config_dir.join("settings.json"),
        })
    }

    // --------------------------------------------------------------------------
    // Track library

    /// Insert many tracks in a single transaction. Returns a Vec of Option<u64>
    /// Some(id) for newly inserted, None if already exists.
    pub fn insert_tracks_batch(
        &self,
        records: &[TrackRecord],
    ) -> Result<Vec<Option<u64>>, Box<dyn std::error::Error>> {
        // First, collect all existing paths in one read transaction.
        let existing: std::collections::HashMap<String, u64> = {
            let tx = self.library.begin_read()?;
            let table = tx.open_table(TRACKS)?;
            let mut map = std::collections::HashMap::new();
            for item in table.iter()? {
                let (k, v) = item?;
                if let Ok(r) = serde_json::from_str::<TrackRecord>(v.value()) {
                    map.insert(r.path.clone(), k.value());
                }
            }
            map
        };

        // One write transaction for all new tracks.
        let tx = self.library.begin_write()?;
        let mut ids: Vec<Option<u64>> = Vec::with_capacity(records.len());
        {
            let mut meta_table = tx.open_table(META)?;
            let mut tracks_table = tx.open_table(TRACKS)?;
            let mut next_id = meta_table.get("next_id")?.map(|v| v.value()).unwrap_or(1);

            for record in records {
                if let Some(&existing_id) = existing.get(&record.path) {
                    ids.push(Some(existing_id)); // already in DB, reuse id
                    continue;
                }
                let id = next_id;
                next_id += 1;
                let mut r = record.clone();
                r.id = id;
                tracks_table.insert(id, serde_json::to_string(&r)?.as_str())?;
                ids.push(Some(id));
            }

            meta_table.insert("next_id", next_id)?;
        }
        tx.commit()?;
        Ok(ids)
    }

    pub fn remove_track(&self, id: u64) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.library.begin_write()?;
        {
            let mut tracks = tx.open_table(TRACKS)?;
            tracks.remove(id)?;
        } // tracks drops here, releasing the borrow on tx
        tx.commit()?;
        Ok(())
    }

    pub fn clear_all_tracks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.library.begin_write()?;
        {
            // delete_table removes all data in one shot, no per-row iteration
            if tx.list_tables()?.any(|t| t.name() == "tracks") {
                tx.delete_table(tx.open_table(TRACKS)?)?;
            }
            // Recreate it empty and reset the id counter
            tx.open_table(TRACKS)?;
            let mut meta = tx.open_table(META)?;
            meta.insert("next_id", 1u64)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn load_tracks_by_ids(
        &self,
        ids: &[u64],
    ) -> Result<Vec<TrackRecord>, Box<dyn std::error::Error>> {
        let tx = self.library.begin_read()?;
        let tracks = tx.open_table(TRACKS)?;
        let mut result = Vec::with_capacity(ids.len());
        for &id in ids {
            if let Some(v) = tracks.get(id)? {
                if let Ok(r) = serde_json::from_str::<TrackRecord>(v.value()) {
                    result.push(r);
                }
            }
        }
        Ok(result)
    }

    pub fn load_all_tracks(&self) -> Result<Vec<TrackRecord>, Box<dyn std::error::Error>> {
        let tx = self.library.begin_read()?;
        let tracks = tx.open_table(TRACKS)?;
        let mut result = Vec::new();
        for item in tracks.iter()? {
            let (_, v) = item?;
            if let Ok(r) = serde_json::from_str::<TrackRecord>(v.value()) {
                result.push(r);
            }
        }
        Ok(result)
    }

    // --------------------------------------------------------------------------
    // App state (playlist order + filters)

    pub fn save_state(&self, state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(state)?;
        // Write to a temp file then rename for atomic replace.
        let tmp = self.state_path.with_extension("json.tmp");
        std::fs::write(&tmp, &json)?;
        std::fs::rename(&tmp, &self.state_path)?;
        Ok(())
    }

    pub fn load_state(&self) -> AppState {
        std::fs::read_to_string(&self.state_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    // --------------------------------------------------------------------------
    // Playback position written every few seconds, very cheap

    /// Non-durable write
    pub fn save_position(
        &self,
        track_id: u64,
        position_secs: f64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = self.playback.begin_write()?;
        {
            let mut table = tx.open_table(PLAYBACK)?;
            table.insert("track_id", track_id)?;
            table.insert("position_secs", position_secs.to_bits())?;
        }
        // Durability::None = no fsync, fastest write, survives normal exit but not a hard power loss
        tx.set_durability(redb::Durability::None)?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_position(&self) -> Option<(u64, f64)> {
        let tx = self.playback.begin_read().ok()?;
        let table = tx.open_table(PLAYBACK).ok()?;
        let track_id = table.get("track_id").ok()??.value();
        let pos_bits = table.get("position_secs").ok()??.value();
        Some((track_id, f64::from_bits(pos_bits)))
    }
}
