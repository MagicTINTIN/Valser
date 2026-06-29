use bevy::prelude::*;
use bevy_egui::egui::style::Selection;
use bevy_egui::egui::{Color32, Stroke, Style, Theme};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use std::time::Duration;

use crate::audio::{AudioCommand, PlaybackInfo, PlaybackState, TrackFinished};
use crate::playlist::{FilterScope, Playlist};

// ---------------------------------------------------------------------------
// UI state
// ---------------------------------------------------------------------------

#[derive(Resource, Default)]
pub struct UiState {
    pub volume: f32,
    pub seeking: bool,
    pub seek_preview: f32,
    pub filter: String,
    pub filter_scope: FilterScope,
    pub show_genre_panel: bool, // toggle sidebar visibility
}

// Systems

fn sync_volume_from_audio(playback_info: Res<PlaybackInfo>, mut ui_state: ResMut<UiState>) {
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

fn setup_custom_style(ctx: &egui::Context) {
    ctx.style_mut_of(Theme::Light, use_light_red_accent);
    ctx.style_mut_of(Theme::Dark, use_dark_red_accent);
}

fn use_light_red_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(180, 30, 20);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(92, 20, 20);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(228, 169, 157),
        stroke: Stroke::new(1.0_f32, Color32::from_rgb(92, 20, 20)),
    };
}

fn use_dark_red_accent(style: &mut Style) {
    style.visuals.hyperlink_color = Color32::from_rgb(222, 105, 105);
    style.visuals.text_cursor.stroke.color = Color32::from_rgb(255, 200, 200);
    style.visuals.selection = Selection {
        bg_fill: Color32::from_rgb(140, 50, 50),
        stroke: Stroke::new(1.0_f32, Color32::from_rgb(255, 200, 200)),
    };
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
    setup_custom_style(&ctx);

    if ui_state.show_genre_panel {
        egui::SidePanel::left("genre_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Genres");
                ui.label(
                    egui::RichText::new("Click to whitelist · shift-click to blacklist")
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.separator();

                let counts = playlist.genre_counts();
                let mut pending_action: Option<(String, bool)> = None; // (genre, is_shift)

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (genre, count) in &counts {
                        let is_white = playlist.genre_whitelist.contains(genre);
                        let is_black = playlist.genre_blacklist.contains(genre);

                        let color = if is_white {
                            egui::Color32::from_rgb(100, 200, 100)
                        } else if is_black {
                            egui::Color32::from_rgb(200, 100, 100)
                        } else {
                            egui::Color32::LIGHT_GRAY
                        };

                        let label =
                            egui::RichText::new(format!("{} ({})", genre, count)).color(color);
                        let response = ui.add(egui::Label::new(label).sense(egui::Sense::click()));

                        if response.clicked() {
                            let shift = ui.input(|i| i.modifiers.shift);
                            pending_action = Some((genre.clone(), shift));
                        }
                    }
                });

                if let Some((genre, shift)) = pending_action {
                    if shift {
                        playlist.toggle_blacklist(&genre);
                    } else {
                        playlist.toggle_whitelist(&genre);
                    }
                }

                if !playlist.genre_whitelist.is_empty() || !playlist.genre_blacklist.is_empty() {
                    ui.separator();
                    if ui.button("Clear filters").clicked() {
                        playlist.genre_whitelist.clear();
                        playlist.genre_blacklist.clear();
                    }
                }
            });
    }

    egui::CentralPanel::default().show(ctx, |ui| {
        // egui::TopBottomPanel::top("header").show(ctx, |ui| {
        // Top bar
        ui.horizontal(|ui| {
            ui.heading("🎵 Valser");
            // ui.horizontal(|ui| {
            //     ui.label("🔍");
            //     ui.text_edit_singleline(&mut ui_state.filter);
            //     if !ui_state.filter.is_empty() && ui.small_button("✖").clicked() {
            //         ui_state.filter.clear();
            //     }
            // });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗀 Add Directory").clicked() {
                    add_directory_action(&mut playlist);
                }
                if ui.button("➕ Add Files").clicked() {
                    add_files_action(&mut playlist);
                }
            });
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("🔍");
            ui.text_edit_singleline(&mut ui_state.filter);
            if !ui_state.filter.is_empty() && ui.small_button("✖").clicked() {
                ui_state.filter.clear();
            }

            egui::ComboBox::from_id_salt("filter_scope")
                .selected_text(match ui_state.filter_scope {
                    FilterScope::TrackName => "Track name",
                    FilterScope::Artist => "Artist",
                    FilterScope::FileName => "Filename",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut ui_state.filter_scope,
                        FilterScope::TrackName,
                        "Track name",
                    );
                    ui.selectable_value(&mut ui_state.filter_scope, FilterScope::Artist, "Artist");
                    ui.selectable_value(
                        &mut ui_state.filter_scope,
                        FilterScope::FileName,
                        "Filename",
                    );
                });

            ui.toggle_value(&mut ui_state.show_genre_panel, "🏷 Genres");
        });

        // ui.separator();
        ui.add(egui::Separator::default().shrink(20_f32));
        // ui.separator().labelled_by("Playlist".into());

        // Playlist
        let available_height = ui.available_height() - 120.0;
        // egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical()
            .max_height(available_height)
            .show(ui, |ui| {
                let mut action: Option<PlaylistAction> = None;

                // let filter_lower = ui_state.filter.to_lowercase();
                for (i, track) in playlist.tracks.iter().enumerate() {
                    // if !filter_lower.is_empty()
                    //     && !track.name.to_lowercase().contains(&filter_lower)
                    // {
                    //     continue;
                    // }
                    if !track.matches_filter(&ui_state.filter, ui_state.filter_scope) {
                        continue;
                    }
                    if !playlist.genre_visible(track) {
                        continue;
                    }
                    let is_current = playlist.current == Some(i);
                    let is_playing = is_current && *playback_state == PlaybackState::Playing;

                    ui.horizontal(|ui| {
                        let indicator = if is_playing {
                            "▶"
                        } else if is_current {
                            "◼"
                        } else {
                            "  "
                        };
                        ui.label(egui::RichText::new(indicator).color(if is_current {
                            egui::Color32::from_rgb(200, 75, 75)
                        } else {
                            egui::Color32::GRAY
                        }));

                        let label =
                            egui::RichText::new(format!("{}. {}", i + 1, track.display_name()))
                                .color(if is_current {
                                    egui::Color32::WHITE
                                } else {
                                    egui::Color32::LIGHT_GRAY
                                });

                        if ui
                            .add(egui::Label::new(label).sense(egui::Sense::click()))
                            .double_clicked()
                        {
                            action = Some(PlaylistAction::Play(i));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new("✖").small())
                                .on_hover_text("Remove")
                                .clicked()
                            {
                                action = Some(PlaylistAction::Remove(i));
                            }
                            if let Some(dur) = track.duration {
                                ui.label(
                                    egui::RichText::new(crate::playlist::Track::format_duration(
                                        dur,
                                    ))
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
                        if was_current {
                            audio_cmd.stop = true;
                        }
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
        // });

        ui.separator();

        // Seek bar
        let total_secs = playback_info
            .duration
            .map(|d| d.as_secs_f32())
            .unwrap_or(0.0);
        let pos_secs = playback_info.position.as_secs_f32();

        let mut seek_val = if ui_state.seeking {
            ui_state.seek_preview
        } else {
            pos_secs
        };

        ui.horizontal(|ui| {
            ui.label(crate::playlist::Track::format_duration(
                Duration::from_secs_f32(pos_secs.max(0.0)),
            ));

            ui.label("/");

            ui.label(crate::playlist::Track::format_duration(
                Duration::from_secs_f32(total_secs),
            ));

            // ui.style_mut().visuals.color

            ui.style_mut().spacing.slider_width = ui.available_width();
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
        });

        // Transport + volume
        ui.horizontal(|ui| {
            if ui
                .add_enabled(playlist.prev_track().is_some(), egui::Button::new("⏮"))
                .on_hover_text("Previous")
                .clicked()
            {
                if let Some(prev) = playlist.prev_track() {
                    let path = playlist.tracks[prev].path.clone();
                    playlist.current = Some(prev);
                    audio_cmd.play = Some(path);
                }
            }

            let play_label = if *playback_state == PlaybackState::Playing {
                "⏸"
            } else {
                "▶"
            };
            if ui
                .button(play_label)
                .on_hover_text("Play / Pause")
                .clicked()
            {
                match *playback_state {
                    PlaybackState::Stopped => {
                        let idx = playlist.current.or_else(|| {
                            if playlist.tracks.is_empty() {
                                None
                            } else {
                                Some(0)
                            }
                        });
                        // println!("Hey {}", idx.unwrap_or(0));
                        if let Some(i) = idx {
                            let path = playlist.tracks[i].path.clone();
                            playlist.current = Some(i);
                            audio_cmd.play = Some(path);
                        }
                    }
                    _ => {
                        audio_cmd.toggle_pause = true;
                    }
                }
            }

            if ui
                .add_enabled(
                    *playback_state != PlaybackState::Stopped,
                    egui::Button::new("⏹"),
                )
                .on_hover_text("Stop")
                .clicked()
            {
                audio_cmd.stop = true;
            }

            if ui
                .add_enabled(playlist.next_track().is_some(), egui::Button::new("⏭"))
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
            if ui
                .add(
                    egui::Slider::new(&mut vol, 0.0..=1.0)
                        .show_value(false)
                        .trailing_fill(true),
                )
                .changed()
            {
                ui_state.volume = vol;
                audio_cmd.volume = Some(vol);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(i) = playlist.current {
                    if let Some(track) = playlist.tracks.get(i) {
                        ui.label(
                            egui::RichText::new(format!("♪ {}", &track.display_name()))
                                .color(egui::Color32::from_rgb(220, 75, 75))
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
// Keyboard shortcuts

fn handle_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut playlist: ResMut<Playlist>,
    mut audio_cmd: ResMut<AudioCommand>,
    playback_state: Res<PlaybackState>,
    playback_info: Res<PlaybackInfo>,
    mut contexts: EguiContexts,
) {
    // Don't steal keystrokes while the user is typing in a text field.
    if let Ok(ctx) = contexts.ctx_mut() {
        if ctx.wants_keyboard_input() {
            return;
        }
    }

    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // Space, or XF86 PlayPause -> toggle play/pause
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::MediaPlayPause) {
        match *playback_state {
            PlaybackState::Stopped => {
                let idx = playlist.current.or(if playlist.tracks.is_empty() {
                    None
                } else {
                    Some(0)
                });
                if let Some(i) = idx {
                    let path = playlist.tracks[i].path.clone();
                    playlist.current = Some(i);
                    audio_cmd.play = Some(path);
                }
            }
            _ => audio_cmd.toggle_pause = true,
        }
        return;
    }
    
    if keys.pressed(KeyCode::ArrowRight) {
        let new_pos = playback_info.position.saturating_add(Duration::from_millis(500));
        audio_cmd.seek = Some(new_pos);
        return;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        let new_pos = playback_info.position.saturating_sub(Duration::from_millis(500));
        audio_cmd.seek = Some(new_pos);
        return;
    }
    
    if keys.just_pressed(KeyCode::KeyJ) {
        let new_pos = playback_info.position.saturating_add(Duration::from_secs(10));
        audio_cmd.seek = Some(new_pos);
        return;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        let new_pos = playback_info.position.saturating_sub(Duration::from_secs(10));
        audio_cmd.seek = Some(new_pos);
        return;
    }

    // XF86 next/previous track keys
    if keys.just_pressed(KeyCode::MediaTrackNext) {
        if let Some(next) = playlist.next_track() {
            let path = playlist.tracks[next].path.clone();
            playlist.current = Some(next);
            audio_cmd.play = Some(path);
        }
        return;
    }
    if keys.just_pressed(KeyCode::MediaTrackPrevious) {
        if let Some(prev) = playlist.prev_track() {
            let path = playlist.tracks[prev].path.clone();
            playlist.current = Some(prev);
            audio_cmd.play = Some(path);
        }
        return;
    }

    // Ctrl+Shift+O -> open folder (recursive)
    if ctrl && shift && keys.just_pressed(KeyCode::KeyO) {
        add_directory_action(&mut playlist);
        return; // avoid matching the plain Ctrl+O branch below
    }

    // Ctrl+O -> open files
    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        add_files_action(&mut playlist);
        return;
    }

    // Ctrl+S -> shuffle
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        playlist.shuffle();
        return;
    }
}

// ---------------------------------------------------------------------------
// Helpers

enum PlaylistAction {
    Play(usize),
    Remove(usize),
}

fn add_files_action(playlist: &mut ResMut<Playlist>) {
    if let Some(paths) = rfd::FileDialog::new()
        .set_title("Add audio files")
        .add_filter(
            "Audio files",
            &["mp3", "ogg", "opus", "flac", "wav", "m4a", "aac", "aiff"],
        )
        .pick_files()
    {
        playlist.add_tracks(paths);
        if playlist.current.is_none() && !playlist.tracks.is_empty() {
            playlist.current = Some(0);
        }
    }
}

fn add_directory_action(playlist: &mut ResMut<Playlist>) {
    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
        playlist.add_directory_recursive(&dir);
        if playlist.current.is_none() && !playlist.tracks.is_empty() {
            playlist.current = Some(0);
        }
    }
}

// --------------------------------------------------------------------------
// Plugin

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Update, sync_volume_from_audio)
            .add_systems(Update, auto_advance)
            .add_systems(Update, handle_shortcuts)
            .add_systems(EguiPrimaryContextPass, draw_ui);
    }
}
