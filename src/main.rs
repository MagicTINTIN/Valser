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
        .add_systems(Startup, setup)
        .insert_non_send_resource(store)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
