use mio_glue::player::{CurrentlyDecoding, Player};
use tokio::sync::oneshot;

use crate::*;

pub fn start_player_thread(
    client: Arc<RwLock<MioClientState>>,
    rt: &tokio::runtime::Runtime,
) -> (
    crossbeam::channel::Sender<DecoderMsg>,
    tokio::sync::watch::Receiver<CurrentlyDecoding>,
) {
    let (tx, rx) = oneshot::channel();
    rt.spawn_blocking(|| player_thread(client, tx));
    rx.blocking_recv().unwrap()
}

fn player_thread(
    client: Arc<RwLock<MioClientState>>,
    ret: oneshot::Sender<(
        crossbeam::channel::Sender<DecoderMsg>,
        tokio::sync::watch::Receiver<CurrentlyDecoding>,
    )>,
) {
    let player = Player::new(client).unwrap();
    ret.send((player.tx.clone(), player.rx.clone()));
    loop {}
}

impl MioFrontendWeak {}
