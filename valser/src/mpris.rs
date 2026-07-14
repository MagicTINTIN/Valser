use bevy::prelude::*;
use mpris_server::{Metadata, PlaybackStatus, Player, Time};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum MprisCommand {
    PlayPause,
    Play,
    Pause,
    Next,
    Previous,
    Stop,
    Seek(Time),
}

pub struct MprisReceiver(pub Receiver<MprisCommand>);

#[derive(Resource, Clone)]
pub struct MprisStateSender(pub Sender<MprisStateUpdate>);

#[derive(Debug, Clone)]
pub enum MprisStateUpdate {
    Playing,
    Paused,
    Stopped,
    Metadata {
        title: String,
        artist: String,
        length_secs: f64,
    },
    Position(f64),
}

pub struct MprisPlugin;

impl Plugin for MprisPlugin {
    fn build(&self, app: &mut App) {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<MprisCommand>();
        let (state_tx, state_rx) = std::sync::mpsc::channel::<MprisStateUpdate>();

        // Spawn the D-Bus server on its own OS thread, using smol
        std::thread::spawn(move || {
            smol::block_on(run_mpris_server(cmd_tx, state_rx));
        });

        app.insert_non_send_resource(MprisReceiver(cmd_rx))
            .insert_resource(MprisStateSender(state_tx))
            .add_systems(Update, drain_mpris_commands);
    }
}

async fn run_mpris_server(cmd_tx: Sender<MprisCommand>, state_rx: Receiver<MprisStateUpdate>) {
    let player = match Player::builder("fr.magictintin.valser")
        .can_play(true)
        .can_pause(true)
        .can_go_next(true)
        .can_go_previous(true)
        .can_seek(true)
        .can_control(true)
        .identity("Valser")
        .build()
        .await
    {
        Ok(p) => {
            info!("MPRIS server started.");
            p
        }
        Err(e) => {
            eprintln!("Failed to start MPRIS server: {e}");
            return;
        }
    };

    {
        let tx = cmd_tx.clone();
        player.connect_play_pause(move |_| {
            let _ = tx.send(MprisCommand::PlayPause);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_play(move |_| {
            let _ = tx.send(MprisCommand::Play);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_pause(move |_| {
            let _ = tx.send(MprisCommand::Pause);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_next(move |_| {
            let _ = tx.send(MprisCommand::Next);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_previous(move |_| {
            let _ = tx.send(MprisCommand::Previous);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_stop(move |_| {
            let _ = tx.send(MprisCommand::Stop);
        });
    }
    {
        let tx = cmd_tx.clone();
        player.connect_seek(move |_, seek_value| {
            let _ = tx.send(MprisCommand::Seek(seek_value));
        });
    }

    let run_fut = player.run();

    let poll_fut = async {
        loop {
            while let Ok(update) = state_rx.try_recv() {
                match update {
                    MprisStateUpdate::Playing => {
                        let _ = player.set_playback_status(PlaybackStatus::Playing).await;
                    }
                    MprisStateUpdate::Paused => {
                        let _ = player.set_playback_status(PlaybackStatus::Paused).await;
                    }
                    MprisStateUpdate::Stopped => {
                        let _ = player.set_playback_status(PlaybackStatus::Stopped).await;
                    }
                    MprisStateUpdate::Metadata {
                        title,
                        artist,
                        length_secs,
                    } => {
                        let metadata = Metadata::builder()
                            .title(title)
                            .artist([artist])
                            .length(Time::from_secs(length_secs as i64))
                            .build();
                        let _ = player.set_metadata(metadata).await;
                    }
                    MprisStateUpdate::Position(secs) => {
                        // counts as activity for playerctld's recency tracking.
                        let _ = player.seeked(Time::from_secs(secs as i64)).await;
                    }
                }
            }
            smol::Timer::after(Duration::from_millis(100)).await;
        }
    };

    futures::future::join(run_fut, poll_fut).await;
}

fn drain_mpris_commands(
    receiver: NonSend<MprisReceiver>,
    mut audio_cmd: ResMut<crate::audio::AudioCommand>,
    mut playlist: ResMut<crate::playlist::Playlist>,
    playback_state: Res<crate::audio::PlaybackState>,
) {
    use crate::audio::PlaybackState;

    while let Ok(cmd) = receiver.0.try_recv() {
        debug!(
            "{}",
            match cmd {
                MprisCommand::PlayPause => "PlayPause",
                MprisCommand::Play => "Play",
                MprisCommand::Pause => "Pause",
                MprisCommand::Next => "Next",
                MprisCommand::Previous => "Previous",
                MprisCommand::Stop => "Stop",
                MprisCommand::Seek(_) => "Seek",
            }
        );
        match cmd {
            MprisCommand::PlayPause => match *playback_state {
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
            },
            MprisCommand::Play => {
                if *playback_state != PlaybackState::Playing {
                    audio_cmd.toggle_pause = true;
                }
            }
            MprisCommand::Pause => {
                if *playback_state == PlaybackState::Playing {
                    audio_cmd.toggle_pause = true;
                }
            }
            MprisCommand::Stop => audio_cmd.stop = true,
            MprisCommand::Next => {
                if let Some(next) = playlist.next_track() {
                    let path = playlist.tracks[next].path.clone();
                    playlist.current = Some(next);
                    audio_cmd.play = Some(path);
                }
            }
            MprisCommand::Previous => {
                if let Some(prev) = playlist.prev_track() {
                    let path = playlist.tracks[prev].path.clone();
                    playlist.current = Some(prev);
                    audio_cmd.play = Some(path);
                }
            }
            MprisCommand::Seek(time_value) => {
                audio_cmd.seek = Some(Duration::from_micros(
                    time_value.as_micros().try_into().unwrap_or(0),
                ));
            }
        }
    }
}
