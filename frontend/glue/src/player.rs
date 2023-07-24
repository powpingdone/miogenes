use crate::*;
use crossbeam::channel::{Receiver, RecvTimeoutError, Sender};
use flutter_rust_bridge::StreamSink;
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub(crate) enum PlayerMsg {
    SetSink(StreamSink<api::PStatus>),
    Play(Option<Uuid>),
    Pause,
    Toggle,
    Queue(Uuid),
    Unqueue(Uuid),
    SeekAbs(f64),
    Stop,
    Drop,
}

enum AudioMsg {
    Preload(Uuid),
    Play(Uuid),
    SeekAbs(f64),
    Pause,
    Stop,
}

#[derive(Debug)]
pub struct Player {
    pub(crate) send: crossbeam::channel::Sender<crate::player::PlayerMsg>,
}

impl Player {
    pub(crate) fn new(client: Arc<RwLock<MioClientState>>) -> Self {
        let (tx_player, rx_player) = crossbeam::channel::unbounded();
        let (tx_audio, rx_audio) = crossbeam::channel::unbounded();

        // I don't like doing this, but i'm not joining these threads. Ignore the dropped
        // handles.
        std::thread::spawn(move || player_track_mgr(tx_audio, rx_player));
        std::thread::spawn({
            let tx_player = tx_player.clone();
            move || player_audio(client, tx_player, rx_audio)
        });
        Self { send: tx_player }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.send.send(PlayerMsg::Drop).unwrap();
    }
}

fn player_track_mgr(tx: Sender<AudioMsg>, rx: Receiver<PlayerMsg>) {
    let mut sink: Option<StreamSink<api::PStatus>> = None;
    loop {
        let recv = rx.recv_deadline(Instant::now() + Duration::from_millis(50));
        if let Ok(msg) = recv {
            match msg {
                PlayerMsg::SetSink(new_sink) => sink = Some(new_sink),
                PlayerMsg::Play(_) => todo!(),
                PlayerMsg::Pause => todo!(),
                PlayerMsg::Toggle => todo!(),
                PlayerMsg::Queue(_) => todo!(),
                PlayerMsg::Unqueue(_) => todo!(),
                PlayerMsg::Stop => todo!(),
                PlayerMsg::Drop => return,
                PlayerMsg::SeekAbs(_) => todo!(),
            }
        }
        sink.as_ref().map(|x| x.add(api::PStatus {}));
    }
}

fn player_audio(
    client: Arc<RwLock<MioClientState>>,
    tx: Sender<PlayerMsg>,
    rx: Receiver<AudioMsg>,
) {
}
