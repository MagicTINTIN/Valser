mod audio;
mod playlist;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use audio::AudioPlugin;
use playlist::PlaylistPlugin;
use ui::UiPlugin;
mod opus_source;

fn main() {
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
        .add_plugins(PlaylistPlugin)
        .add_plugins(UiPlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
