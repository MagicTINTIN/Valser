use crossbeam_channel::Sender;
use mpris_server::{Metadata, PlaybackStatus, Player, Time};
use smol::channel::Receiver;
use valser_core::audio::PlaybackState;

use crate::engine::{Cmd, MediaState};

/// Runs the D-Bus server on its own thread.
pub fn spawn(tx: Sender<Cmd>) -> smol::channel::Sender<MediaState> {
    let (state_tx, state_rx) = smol::channel::unbounded();
    std::thread::spawn(move || smol::block_on(run(tx, state_rx)));
    state_tx
}

fn forward(tx: &Sender<Cmd>, cmd: fn() -> Cmd) -> impl Fn(&Player) + 'static {
    let tx = tx.clone();
    move |_| {
        let _ = tx.send(cmd());
    }
}

fn status(state: PlaybackState) -> PlaybackStatus {
    match state {
        PlaybackState::Playing => PlaybackStatus::Playing,
        PlaybackState::Paused => PlaybackStatus::Paused,
        PlaybackState::Stopped => PlaybackStatus::Stopped,
    }
}

async fn run(tx: Sender<Cmd>, state_rx: Receiver<MediaState>) {
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
        Ok(player) => player,
        Err(e) => return eprintln!("Failed to start MPRIS server: {e}"),
    };

    player.connect_play_pause(forward(&tx, || Cmd::Toggle));
    player.connect_play(forward(&tx, || Cmd::Resume));
    player.connect_pause(forward(&tx, || Cmd::Pause));
    player.connect_next(forward(&tx, || Cmd::Next));
    player.connect_previous(forward(&tx, || Cmd::Prev));
    player.connect_stop(forward(&tx, || Cmd::Stop));
    player.connect_seek(move |_, offset| {
        let secs = offset.as_micros() as f64 / 1e6;
        let _ = tx.send(Cmd::SeekBy { secs });
    });

    let updates = async {
        let mut last: Option<MediaState> = None;
        while let Ok(s) = state_rx.recv().await {
            if last.as_ref().is_none_or(|l| l.state != s.state) {
                let _ = player.set_playback_status(status(s.state)).await;
            }
            if last
                .as_ref()
                .is_none_or(|l| l.title != s.title || l.artist != s.artist || l.length != s.length)
            {
                let metadata = Metadata::builder()
                    .title(s.title.clone())
                    .artist([s.artist.clone()])
                    .length(Time::from_secs(s.length as i64))
                    .build();
                let _ = player.set_metadata(metadata).await;
            }
            if s.state == PlaybackState::Playing {
                // Also counts as activity for playerctld
                let _ = player.seeked(Time::from_secs(s.position as i64)).await;
            }
            last = Some(s);
        }
    };

    smol::future::zip(player.run(), updates).await;
}
