use bevy::prelude::*;
use bevy_egui::egui::style::Selection;
use bevy_egui::egui::{Color32, Stroke, Style, Theme};
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};
use std::path::PathBuf;
use std::time::Duration;

use crate::audio::{AudioCommand, PlaybackInfo, PlaybackState, TrackFinished};
use crate::loader::{LoadRequest, LoaderChannel, LoadingState};
use crate::playlist::{FilterScope, Playlist, Track};

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
    store: NonSend<crate::store::Store>,
    loader: Option<Res<LoaderChannel>>,
    mut loading: Option<ResMut<LoadingState>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    setup_custom_style(&ctx);

    let mut genre_action: Option<(String, bool)> = None;
    let mut clear_genre_filters = false;
    let mut playlist_action: Option<PlaylistAction> = None;
    let mut save_state_needed = false;

    // -----------------------------------------------------------------------
    // Genre side panel
    if ui_state.show_genre_panel {
        let counts = playlist.genre_counts();
        let whitelist_snap = playlist.genre_whitelist.clone();
        let blacklist_snap = playlist.genre_blacklist.clone();

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

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (genre, count) in &counts {
                        let is_white = whitelist_snap.contains(genre);
                        let is_black = blacklist_snap.contains(genre);

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
                            genre_action = Some((genre.clone(), shift));
                        }
                    }
                });

                if !whitelist_snap.is_empty() || !blacklist_snap.is_empty() {
                    ui.separator();
                    if ui.button("Clear filters").clicked() {
                        clear_genre_filters = true;
                    }
                }
            });
    }

    // -----------------------------------------------------------------------
    // Central panel
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("🎵 Valser");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🗀 Add Directory")
                    .on_hover_text("Add directory to playlist (Ctrl+Shift+O)").clicked() {
                    if let (Some(loader), Some(loading)) = (&loader, &mut loading) {
                        add_directory_action(loader, loading);
                    }
                }
                if ui.button("➕ Add Files")
                    .on_hover_text("Add tracks to playlist (Ctrl+O)").clicked() {
                    if let (Some(loader), Some(loading)) = (&loader, &mut loading) {
                        add_files_action(loader, loading);
                    }
                }
                if ui
                    .button("🗑 Clear")
                    .clicked()
                {
                    let _ = store.clear_all_tracks();
                    playlist.tracks.clear();
                    playlist.current = None;
                    audio_cmd.stop = true;
                    save_state_needed = true;
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
        if let Some(ref loading) = loading {
            if loading.is_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(loading.status_text());
                    ui.add(egui::ProgressBar::new(loading.progress()).desired_width(200.0));
                });
            }
        }

        ui.add(egui::Separator::default().shrink(20_f32));

        // Snapshot visible tracks, indices into playlist.tracks
        let visible_tracks: Vec<(usize, String, Option<std::time::Duration>, bool, bool)> =
            playlist
                .tracks
                .iter()
                .enumerate()
                .filter(|(_, t)| {
                    t.matches_filter(&ui_state.filter, ui_state.filter_scope)
                        && playlist.genre_visible(t)
                })
                .map(|(i, t)| {
                    let is_current = playlist.current == Some(i);
                    let is_playing = is_current && *playback_state == PlaybackState::Playing;
                    (
                        i,
                        t.display_name().to_string(),
                        t.duration,
                        is_current,
                        is_playing,
                    )
                })
                .collect();

        let available_height = ui.available_height() - 120.0;
        let row_height = 22.0;

        egui::ScrollArea::vertical()
            .max_height(available_height)
            .show_rows(ui, row_height, visible_tracks.len(), |ui, row_range| {
                for idx in row_range {
                    let (i, display_name, duration, is_current, is_playing) = &visible_tracks[idx];

                    ui.horizontal(|ui| {
                        let indicator = if *is_playing {
                            "▶"
                        } else if *is_current {
                            "◼"
                        } else {
                            "  "
                        };
                        ui.label(egui::RichText::new(indicator).color(if *is_current {
                            egui::Color32::from_rgb(200, 75, 75)
                        } else {
                            egui::Color32::GRAY
                        }));

                        let label = egui::RichText::new(format!("{}. {}", i + 1, display_name))
                            .color(if *is_current {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::LIGHT_GRAY
                            });

                        if ui
                            .add(egui::Label::new(label).sense(egui::Sense::click()))
                            .double_clicked()
                        {
                            playlist_action = Some(PlaylistAction::Play(*i));
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new("✖").small())
                                .on_hover_text("Remove")
                                .clicked()
                            {
                                playlist_action = Some(PlaylistAction::Remove(*i));
                            }
                            if let Some(dur) = duration {
                                ui.label(
                                    egui::RichText::new(Track::format_duration(*dur))
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        });
                    });
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
            ui.label(Track::format_duration(Duration::from_secs_f32(
                pos_secs.max(0.0),
            )));
            ui.label("/");
            ui.label(Track::format_duration(Duration::from_secs_f32(total_secs)));
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
                        if let Some(i) = idx {
                            let path = playlist.tracks[i].path.clone();
                            playlist.current = Some(i);
                            audio_cmd.play = Some(path);
                        }
                    }
                    _ => audio_cmd.toggle_pause = true,
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
                            egui::RichText::new(format!("♪ {}", track.display_name()))
                                .color(egui::Color32::from_rgb(220, 75, 75))
                                .small(),
                        );
                    }
                }
            });
        });
    });

    // -----------------------------------------------------------------------
    // Apply deferred actions

    if let Some((genre, shift)) = genre_action {
        if shift {
            playlist.toggle_blacklist(&genre);
        } else {
            playlist.toggle_whitelist(&genre);
        }
        save_state_needed = true;
    }

    if clear_genre_filters {
        playlist.genre_whitelist.clear();
        playlist.genre_blacklist.clear();
        save_state_needed = true;
    }

    match playlist_action {
        Some(PlaylistAction::Play(i)) => {
            let path = playlist.tracks[i].path.clone();
            playlist.current = Some(i);
            audio_cmd.play = Some(path);
        }
        Some(PlaylistAction::Remove(i)) => {
            let was_current = playlist.current == Some(i);
            if let Some(track) = playlist.tracks.get(i) {
                let _ = store.remove_track(track.id);
            }
            playlist.remove_track(i);
            if was_current {
                audio_cmd.stop = true;
            }
            save_state_needed = true;
        }
        None => {}
    }

    if save_state_needed {
        let _ = store.save_state(&build_app_state(&playlist, &ui_state));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Keyboard shortcuts

fn handle_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut playlist: ResMut<Playlist>,
    mut audio_cmd: ResMut<AudioCommand>,
    ui_state: ResMut<UiState>,
    playback_state: Res<PlaybackState>,
    playback_info: Res<PlaybackInfo>,
    mut contexts: EguiContexts,
    store: NonSend<crate::store::Store>,
    loader: Option<Res<LoaderChannel>>,
    mut loading: Option<ResMut<LoadingState>>,
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
        let new_pos = playback_info
            .position
            .saturating_add(Duration::from_millis(500));
        audio_cmd.seek = Some(new_pos);
        return;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        let new_pos = playback_info
            .position
            .saturating_sub(Duration::from_millis(500));
        audio_cmd.seek = Some(new_pos);
        return;
    }

    if keys.just_pressed(KeyCode::KeyJ) {
        let new_pos = playback_info
            .position
            .saturating_add(Duration::from_secs(10));
        audio_cmd.seek = Some(new_pos);
        return;
    }
    if keys.just_pressed(KeyCode::KeyL) {
        let new_pos = playback_info
            .position
            .saturating_sub(Duration::from_secs(10));
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
        // add_directory_action(&mut playlist, &*store, &ui_state);
        if let (Some(loader), Some(loading)) = (&loader, &mut loading) {
            add_directory_action(loader, loading);
        }
        return; // avoid matching the plain Ctrl+O branch below
    }

    // Ctrl+O -> open files
    if ctrl && keys.just_pressed(KeyCode::KeyO) {
        // add_files_action(&mut playlist, &*store, &ui_state);
        if let (Some(loader), Some(loading)) = (&loader, &mut loading) {
            add_files_action(loader, loading);
        }
        return;
    }

    // Ctrl+S -> shuffle
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        playlist.shuffle();
        let _ = store.save_state(&build_app_state(&playlist, &ui_state));
        return;
    }
}

// ---------------------------------------------------------------------------
// Helpers

enum PlaylistAction {
    Play(usize),
    Remove(usize),
}

fn add_files_action(
    loader: &LoaderChannel,
    loading: &mut LoadingState,
) {
    if let Some(paths) = rfd::FileDialog::new()
        .set_title("Add audio files")
        .add_filter(
            "Audio files",
            &["mp3", "ogg", "opus", "flac", "wav", "m4a", "aac", "aiff"],
        )
        .pick_files()
    {
        loading.queued += paths.len();
        loading.loaded = 0;
        loading.is_loading = true;
        let _ = loader.request_tx.send(LoadRequest::Paths(paths));
    }
}

fn add_directory_action(loader: &LoaderChannel, loading: &mut LoadingState) {
    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
        // Collect paths synchronously
        // then hand off everything else to the worker.
        let paths: Vec<PathBuf> = walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| crate::playlist::is_supported_format(p))
            .collect();
        loading.queued = paths.len();
        loading.loaded = 0;
        loading.is_loading = true;
        let _ = loader.request_tx.send(LoadRequest::Paths(paths));
    }
}

fn build_app_state(playlist: &Playlist, ui_state: &UiState) -> crate::store::AppState {
    crate::store::AppState {
        playlist_order: playlist.tracks.iter().map(|t| t.id).collect(),
        current_index: playlist.current,
        filter_text: ui_state.filter.clone(),
        filter_scope: match ui_state.filter_scope {
            FilterScope::TrackName => "name".to_string(),
            FilterScope::Artist => "artist".to_string(),
            FilterScope::FileName => "filename".to_string(),
        },
        genre_whitelist: playlist.genre_whitelist.iter().cloned().collect(),
        genre_blacklist: playlist.genre_blacklist.iter().cloned().collect(),
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
