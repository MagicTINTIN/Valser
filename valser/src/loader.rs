use crossbeam_channel::{unbounded, Receiver, Sender};
use std::path::PathBuf;
use bevy::prelude::*;

use crate::playlist::Track;
use crate::store::TrackRecord;

// ---------------------------------------------------------------------------
// Messages

pub enum LoadRequest {
    Paths(Vec<PathBuf>),
}

pub enum LoadResult {
    /// Tag reading done for one track, ready to insert into DB.
    Record(TrackRecord),
    BatchDone,
}

#[derive(Resource)]
pub struct LoaderChannel {
    pub request_tx: Sender<LoadRequest>,
    pub result_rx:  Receiver<LoadResult>,
}

#[derive(Resource, Default)]
pub struct LoadingState {
    pub is_loading: bool,
    pub queued:     usize,
    pub loaded:     usize,
}

impl LoadingState {
    pub fn progress(&self) -> f32 {
        if self.queued == 0 { return 1.0; }
        self.loaded as f32 / self.queued as f32
    }
    pub fn status_text(&self) -> String {
        if !self.is_loading { return String::new(); }
        format!("Loading… {}/{}", self.loaded, self.queued)
    }
}

// ---------------------------------------------------------------------------
// Worker: only reads tags, no DB access

pub fn spawn_loader_thread() -> LoaderChannel {
    let (request_tx, request_rx) = unbounded::<LoadRequest>();
    let (result_tx,  result_rx)  = unbounded::<LoadResult>();

    std::thread::spawn(move || {
        for request in &request_rx {
            match request {
                LoadRequest::Paths(paths) => {
                    for path in paths {
                        let track = Track::new(path);  // lofty tag read here
                        let _ = result_tx.send(LoadResult::Record(track.to_record()));
                    }
                    let _ = result_tx.send(LoadResult::BatchDone);
                }
            }
        }
    });

    LoaderChannel { request_tx, result_rx }
}

// ---------------------------------------------------------------------------
// Bevy system: drains results, does the DB write on the main thread

pub fn receive_loaded_tracks(
    channel:    Res<LoaderChannel>,
    mut playlist: ResMut<crate::playlist::Playlist>,
    mut loading:  ResMut<LoadingState>,
    store:      NonSend<crate::store::Store>,
) {
    // Accumulate records to batch-insert at end of drain
    let mut pending: Vec<TrackRecord> = Vec::new();

    for _ in 0..500 {
        match channel.result_rx.try_recv() {
            Ok(LoadResult::Record(record)) => {
                pending.push(record);
                loading.loaded += 1;
            }
            Ok(LoadResult::BatchDone) => {
                loading.is_loading = false;
                // fall through to insert pending below
                break;
            }
            Err(_) => break, // channel empty this frame
        }
    }

    // Batch insert whatever arrived this frame
    if !pending.is_empty() {
        match store.insert_tracks_batch(&pending) {
            Ok(ids) => {
                for (record, id) in pending.into_iter().zip(ids.into_iter()) {
                    if let Some(id) = id {
                        if !playlist.tracks.iter().any(|t| t.id == id) {
                            let mut track = Track::from_record(record);
                            track.id = id;
                            playlist.tracks.push(track);
                        }
                    }
                }
            }
            Err(e) => bevy::log::error!("Batch insert failed: {e}"),
        }

        if playlist.current.is_none() && !playlist.tracks.is_empty() {
            playlist.current = Some(0);
        }
    }
}

// ---------------------------------------------------------------------------
// Plugin

pub struct LoaderPlugin;

impl Plugin for LoaderPlugin {
    fn build(&self, app: &mut App) {
        let channel = spawn_loader_thread();
        app.insert_resource(channel)
           .init_resource::<LoadingState>()
           .add_systems(Update, receive_loaded_tracks);
    }
}