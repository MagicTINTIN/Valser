mod audio;
mod mpris;
mod playlist;
mod ui;

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use audio::AudioPlugin;
use dirs;
use mpris::MprisPlugin;
use playlist::PlaylistPlugin;
use ui::UiPlugin;
mod opus_source;
mod store;
use store::Store;

use crate::playlist::{FilterScope, Playlist, Track};

fn restore_state(
    store: NonSend<store::Store>,
    mut playlist: ResMut<Playlist>,
    mut ui_state: ResMut<ui::UiState>,
) {
    let state = store.load_state();

    if !state.playlist_order.is_empty() {
        if let Ok(records) = store.load_tracks_by_ids(&state.playlist_order) {
            let mut by_id: std::collections::HashMap<u64, _> =
                records.into_iter().map(|r| (r.id, r)).collect();
            for id in &state.playlist_order {
                if let Some(record) = by_id.remove(id) {
                    playlist.tracks.push(Track::from_record(record));
                }
            }
        }
    }

    playlist.genre_whitelist = state.genre_whitelist.into_iter().collect();
    playlist.genre_blacklist = state.genre_blacklist.into_iter().collect();
    ui_state.filter = state.filter_text;
    // restore filter_scope from the string
    ui_state.filter_scope = match state.filter_scope.as_str() {
        "artist"   => FilterScope::Artist,
        "filename" => FilterScope::FileName,
        _          => FilterScope::TrackName,
    };

    // Restore current track index
    playlist.current = state.current_index;
}


fn main() {
    let data_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Valser");
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Valser");

    let store = match Store::open(&data_dir, &config_dir) {
        Ok(s) => s,
        Err(e) => { eprintln!("Failed to open store: {e}"); std::process::exit(1); }
    };

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Valser".to_string(),
                resolution: (800u32, 500u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(AudioPlugin)
        .add_plugins(MprisPlugin)
        .add_plugins(PlaylistPlugin)
        .add_plugins(UiPlugin)
        .add_systems(Startup, (setup, restore_state).chain())
        .insert_non_send_resource(store)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
