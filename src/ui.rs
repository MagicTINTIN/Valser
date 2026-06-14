use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use std::path::PathBuf;
use std::time::Duration;

use crate::audio::{AudioCommand, PlaybackInfo, PlaybackState, TrackFinished};
use crate::playlist::Playlist;

// ---------------------------------------------------------------------------
// UI state
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct UiState {
    pub volume: f32,
    pub seeking: bool,
    pub seek_preview: f32,
}

// Systems

fn sync_volume_from_audio(
    playback_info: Res<PlaybackInfo>,
    mut ui_state: ResMut<UiState>,
) {
    if !ui_state.seeking {
        ui_state.volume = playback_info.volume;
    }
}

/// Auto-advance to the next track when the current one finishes.
fn auto_advance(
    mut track_finished: MessageReader<TrackFinished>,
    mut playlist: ResMut<Playlist>,
    mut audio_cmd: ResMut<AudioCommand>,
    mut playback_state: ResMut<PlaybackState>,
) {
    for _ in track_finished.read() {
        if let Some(next) = playlist.next_track() {
            let path = playlist.tracks[next].path.clone();
            playlist.current = Some(next);
            audio_cmd.play = Some(path);
        } else {
            *playback_state = PlaybackState::Stopped;
        }
    }
}

/// The main egui draw system.
fn draw_ui(
    mut contexts: EguiContexts,
    mut playlist: ResMut<Playlist>,
    mut audio_cmd: ResMut<AudioCommand>,
    mut ui_state: ResMut<UiState>,
    playback_info: Res<PlaybackInfo>,
    playback_state: Res<PlaybackState>,
) -> Result {
    let ctx = contexts.ctx_mut()?;

    egui::CentralPanel::default().show(ctx, |ui| {
        // Top bar
        ui.horizontal(|ui| {
            ui.heading("🎵 Valser");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("➕ Add Files").clicked() {
                    if let Some(paths) = pick_audio_files() {
                        playlist.add_tracks(paths);
                        if playlist.current.is_none() && !playlist.tracks.is_empty() {
                            playlist.current = Some(0);
                        }
                    }
                }
            });
        });

        ui.separator();

        // Playlist
        let available_height = ui.available_height() - 120.0;
        egui::ScrollArea::vertical()
            .max_height(available_height)
            .show(ui, |ui| {
                let mut action: Option<PlaylistAction> = None;

                for (i, track) in playlist.tracks.iter().enumerate() {
                    let is_current = playlist.current == Some(i);
                    let is_playing = is_current && *playback_state == PlaybackState::Playing;

                    ui.horizontal(|ui| {
                        let indicator = if is_playing { "▶" } else if is_current { "◼" } else { "  " };
                        ui.label(egui::RichText::new(indicator).color(
                            if is_current { egui::Color32::from_rgb(100, 200, 100) }
                            else { egui::Color32::GRAY },
                        ));

                        let label = egui::RichText::new(format!("{}. {}", i + 1, &track.name))
                            .color(if is_current { egui::Color32::WHITE } else { egui::Color32::LIGHT_GRAY });

                        if ui.add(egui::Label::new(label).sense(egui::Sense::click()))
                            .double_clicked()
                        {
                            action = Some(PlaylistAction::Play(i));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new("✖").small())
                                .on_hover_text("Remove")
                                .clicked()
                            {
                                action = Some(PlaylistAction::Remove(i));
                            }
                            if let Some(dur) = track.duration {
                                ui.label(
                                    egui::RichText::new(crate::playlist::Track::format_duration(dur))
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        });
                    });
                }

                match action {
                    Some(PlaylistAction::Play(i)) => {
                        let path = playlist.tracks[i].path.clone();
                        playlist.current = Some(i);
                        audio_cmd.play = Some(path);
                    }
                    Some(PlaylistAction::Remove(i)) => {
                        let was_current = playlist.current == Some(i);
                        playlist.remove_track(i);
                        if was_current { audio_cmd.stop = true; }
                    }
                    None => {}
                }

                if playlist.tracks.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(
                            egui::RichText::new("No tracks. Click ➕ Add Files to get started.")
                                .color(egui::Color32::GRAY),
                        );
                    });
                }
            });

        ui.separator();

        // Seek bar
        let total_secs = playback_info.duration.map(|d| d.as_secs_f32()).unwrap_or(0.0);
        let pos_secs = playback_info.position.as_secs_f32();

        let mut seek_val = if ui_state.seeking { ui_state.seek_preview } else { pos_secs };

        ui.horizontal(|ui| {
            ui.label(crate::playlist::Track::format_duration(
                Duration::from_secs_f32(pos_secs.max(0.0)),
            ));

            let seek_slider = ui.add_enabled(
                total_secs > 0.0,
                egui::Slider::new(&mut seek_val, 0.0..=total_secs.max(1.0))
                    .show_value(false)
                    .trailing_fill(true),
            );

            if seek_slider.dragged() {
                ui_state.seeking = true;
                ui_state.seek_preview = seek_val;
            }
            if seek_slider.drag_stopped() {
                ui_state.seeking = false;
                audio_cmd.seek = Some(Duration::from_secs_f32(seek_val));
            }

            ui.label(crate::playlist::Track::format_duration(
                Duration::from_secs_f32(total_secs),
            ));
        });

        // Transport + volume
        ui.horizontal(|ui| {
            if ui.add_enabled(playlist.prev_track().is_some(), egui::Button::new("⏮"))
                .on_hover_text("Previous")
                .clicked()
            {
                if let Some(prev) = playlist.prev_track() {
                    let path = playlist.tracks[prev].path.clone();
                    playlist.current = Some(prev);
                    audio_cmd.play = Some(path);
                }
            }

            let play_label = if *playback_state == PlaybackState::Playing { "⏸" } else { "▶" };
            if ui.button(play_label).on_hover_text("Play / Pause").clicked() {
                match *playback_state {
                    PlaybackState::Stopped => {
                        let idx = playlist.current.or_else(|| {
                            if playlist.tracks.is_empty() { None } else { Some(0) }
                        });
                        // println!("Hey {}", idx.unwrap_or(0));
                        if let Some(i) = idx {
                            let path = playlist.tracks[i].path.clone();
                            playlist.current = Some(i);
                            audio_cmd.play = Some(path);
                        }
                    }
                    _ => { audio_cmd.toggle_pause = true; }
                }
            }

            if ui.add_enabled(*playback_state != PlaybackState::Stopped, egui::Button::new("⏹"))
                .on_hover_text("Stop")
                .clicked()
            {
                audio_cmd.stop = true;
            }

            if ui.add_enabled(playlist.next_track().is_some(), egui::Button::new("⏭"))
                .on_hover_text("Next")
                .clicked()
            {
                if let Some(next) = playlist.next_track() {
                    let path = playlist.tracks[next].path.clone();
                    playlist.current = Some(next);
                    audio_cmd.play = Some(path);
                }
            }

            ui.add_space(16.0);
            ui.label("🔊");

            let mut vol = ui_state.volume;
            if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false).trailing_fill(true))
                .changed()
            {
                ui_state.volume = vol;
                audio_cmd.volume = Some(vol);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(i) = playlist.current {
                    if let Some(track) = playlist.tracks.get(i) {
                        ui.label(
                            egui::RichText::new(format!("♪ {}", &track.name))
                                .color(egui::Color32::from_rgb(150, 200, 255))
                                .small(),
                        );
                    }
                }
            });
        });
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers

enum PlaylistAction { Play(usize), Remove(usize) }

fn pick_audio_files() -> Option<Vec<PathBuf>> {
    rfd::FileDialog::new()
        .set_title("Add audio files")
        .add_filter("Audio files", &["mp3", "ogg", "opus", "flac", "wav", "m4a", "aac", "aiff"])
        .pick_files()
}

// --------------------------------------------------------------------------
// Plugin

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Update, sync_volume_from_audio)
            .add_systems(Update, auto_advance)
            .add_systems(EguiPrimaryContextPass, draw_ui);
    }
}
