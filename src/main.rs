mod audio;
mod loader;
mod mpris;
mod playlist;
mod ui;

use std::{path::PathBuf, time::Duration};

use bevy::{prelude::*, winit::{UpdateMode, WinitSettings}};
use bevy_egui::EguiPlugin;

use audio::AudioPlugin;
use dirs;
use mpris::MprisPlugin;
use playlist::PlaylistPlugin;
use ui::UiPlugin;
mod opus_source;
mod store;
use loader::LoaderPlugin;
use store::Store;

use crate::{
    audio::AudioCommand,
    playlist::{FilterScope, Playlist, Track},
};

fn restore_state(
    store: NonSend<store::Store>,
    mut playlist: ResMut<Playlist>,
    mut ui_state: ResMut<ui::UiState>,
    mut audio_cmd: ResMut<AudioCommand>,
) {
    let state = store.load_state();

    if !state.playlist_order.is_empty() {
        // restore in saved order (post-shuffle etc.)
        if let Ok(records) = store.load_tracks_by_ids(&state.playlist_order) {
            let mut by_id: std::collections::HashMap<u64, _> =
                records.into_iter().map(|r| (r.id, r)).collect();
            for id in &state.playlist_order {
                if let Some(record) = by_id.remove(id) {
                    playlist.tracks.push(Track::from_record(record));
                }
            }
        }
    } else {
        // first launch or state was cleared, load all known tracks from library
        if let Ok(records) = store.load_all_tracks() {
            for record in records {
                playlist.tracks.push(Track::from_record(record));
            }
        }
    }

    playlist.genre_whitelist = state.genre_whitelist.into_iter().collect();
    playlist.genre_blacklist = state.genre_blacklist.into_iter().collect();
    ui_state.filter = state.filter_text;
    // restore filter_scope from the string
    ui_state.filter_scope = match state.filter_scope.as_str() {
        "artist" => FilterScope::Artist,
        "filename" => FilterScope::FileName,
        _ => FilterScope::TrackName,
    };

    // Restore current track index
    playlist.current = state.current_index;

    if let Some((track_id, position_secs)) = store.load_position() {
        if let Some(idx) = playlist.tracks.iter().position(|t| t.id == track_id) {
            playlist.current = Some(idx);
            audio_cmd.seek = Some(Duration::from_secs_f64(position_secs));
        }
    }
}

fn save_state_on_exit(
    mut exit_events: MessageReader<bevy::app::AppExit>,
    store: NonSend<store::Store>,
    playlist: Res<Playlist>,
    ui_state: Res<ui::UiState>,
) {
    for _ in exit_events.read() {
        let state = crate::store::AppState {
            playlist_order: playlist.tracks.iter().map(|t| t.id).collect(),
            current_index: playlist.current,
            filter_text: ui_state.filter.clone(),
            filter_scope: match ui_state.filter_scope {
                FilterScope::Artist => "artist".to_string(),
                FilterScope::FileName => "filename".to_string(),
                _ => "name".to_string(),
            },
            genre_whitelist: playlist.genre_whitelist.iter().cloned().collect(),
            genre_blacklist: playlist.genre_blacklist.iter().cloned().collect(),
        };
        let _ = store.save_state(&state);
    }
}

fn main() {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Valser");
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Valser");

    let store = match Store::open(&data_dir, &config_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open store: {e}");
            std::process::exit(1);
        }
    };

    App::new()
        .insert_resource(WinitSettings {
            // Render at max 30fps when focused, drop to 10fps when unfocused.
            focused_mode: UpdateMode::reactive(Duration::from_millis(33)),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(100)),
        })
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
        .add_plugins(LoaderPlugin)
        .add_systems(Startup, (setup, restore_state).chain())
        .add_systems(Last, save_state_on_exit)
        .insert_non_send_resource(store)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
