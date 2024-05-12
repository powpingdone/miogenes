use super::{CurrentlyDecoding, DecoderMsg};
use crate::player::decoder::ControllingDecoder;
use crate::*;
use crossbeam::channel::{Receiver, RecvTimeoutError};
use log::*;
use parking_lot::Mutex;
use rodio::Source;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(crate) enum PlayerMsg {
    Play(Option<Uuid>),
    Pause,
    Toggle,
    Queue(Uuid),
    Stop,
    Forward,
    Backward,
    Seek(Duration),
}

#[derive(Debug)]
pub struct Player {
    pub(crate) tx: crossbeam::channel::Sender<crate::player::PlayerMsg>,
}

impl Player {
    pub(crate) fn new(client: Arc<RwLock<MioClientState>>) -> Self {
        let (tx_player, rx_player) = crossbeam::channel::unbounded();

        // thread does not get joined due to if tx_player gets dropped, then everything
        // else will die as well
        std::thread::Builder::new()
            .name("MioPlayerT".to_owned())
            .spawn(move || player_track_mgr(client, rx_player))
            .unwrap();
        Self { tx: tx_player }
    }
}

fn player_track_mgr(client: Arc<RwLock<MioClientState>>, rx: Receiver<PlayerMsg>) {
    trace!("opening track manager");
    let mut state = PlayerState::new(client.clone()).unwrap();
    trace!("entering event loop");
    loop {
        let recv = rx.recv_deadline(Instant::now() + Duration::from_millis(50));
        match recv {
            Ok(msg) => match msg {
                PlayerMsg::Play(id) => {
                    if let Some(id) = id {
                        state.yell_to_decoder.send(DecoderMsg::Reset).unwrap();
                        state.yell_to_decoder.send(DecoderMsg::Enqueue(id)).unwrap();
                    }
                    state.yell_to_decoder.send(DecoderMsg::Play).unwrap();
                }
                PlayerMsg::Pause => state.yell_to_decoder.send(DecoderMsg::Pause).unwrap(),
                PlayerMsg::Toggle => {
                    todo!()
                }
                PlayerMsg::Queue(id) => {
                    state.yell_to_decoder.send(DecoderMsg::Enqueue(id)).unwrap()
                }
                PlayerMsg::Stop => state.yell_to_decoder.send(DecoderMsg::Stop).unwrap(),
                PlayerMsg::Forward => state.yell_to_decoder.send(DecoderMsg::Next).unwrap(),
                PlayerMsg::Backward => state.yell_to_decoder.send(DecoderMsg::Previous).unwrap(),
                PlayerMsg::Seek(dur) => state
                    .yell_to_decoder
                    .send(DecoderMsg::SeekAbs(dur))
                    .unwrap(),
            },
            Err(err) if err == RecvTimeoutError::Disconnected => return,
            Err(err) if err == RecvTimeoutError::Timeout => (),
            _ => unreachable!(),
        }

        // get queue back
        let full = state.ret_status.lock();
        let queue: Vec<_> = full.tracks.iter().map(|x| x.id).collect();
        let curr_playing = if full.curr.is_nil() {
            None
        } else {
            Some(full.curr)
        };
        let playback_pos_s = full.at.as_secs();
        let playback_pos_ms = full.at.subsec_millis();
        let playback_len_s = full.len.as_secs();
        let playback_len_ms = full.len.subsec_millis();
        drop(full);

        // add to radio queue
        if queue.len() < 50 {
            let next = client
                .read()
                .unwrap()
                .get_closest(queue[0], queue.clone())
                .unwrap()
                .id;

            // next iteration will pickup the new id in the queue
            state
                .yell_to_decoder
                .send(DecoderMsg::Enqueue(next))
                .unwrap();
        }
    }
}

struct PlayerState {
    _dev: rodio::OutputStream,
    _s_handle: rodio::OutputStreamHandle,
    _dec_thread: std::thread::JoinHandle<()>,
    pub ret_status: Arc<Mutex<CurrentlyDecoding>>,
    pub yell_to_decoder: std::sync::mpsc::Sender<DecoderMsg>,
}

impl PlayerState {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> anyhow::Result<Self> {
        trace!("acqiring dev");
        let (_dev, s_handle) = find_dev()?;
        trace!("setting up decoder");
        let ret_status = Arc::new(Mutex::new(CurrentlyDecoding {
            tracks: vec![],
            curr: Uuid::nil(),
            at: Duration::from_secs(0),
            len: Duration::from_secs(0),
        }));
        let (tx, rx) = std::sync::mpsc::channel();
        let decoder = ControllingDecoder::new(client, ret_status.clone(), rx);
        Ok(Self {
            _dec_thread: std::thread::spawn({
                let s_handle = s_handle.clone();
                move || {
                    trace!("spinning s_thread");
                    s_handle.play_raw(decoder).unwrap();
                }
            }),
            _dev,
            ret_status,
            yell_to_decoder: tx,
            _s_handle: s_handle,
        })
    }
}

fn find_dev() -> anyhow::Result<(rodio::OutputStream, rodio::OutputStreamHandle)> {
    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    )))]
    {
        use std::panic::catch_unwind;

        debug!("not on linux: attempting to get default device");
        Ok({
            let x = catch_unwind(rodio::OutputStream::try_default);
            if let Err(ref err) = x {
                return Err(anyhow::anyhow!("panicked: {:?}", {
                    if let Some(x) = err.downcast_ref::<&str>() {
                        Some(*x)
                    } else {
                        err.downcast_ref::<String>().map(|x| x.as_str())
                    }
                }));
            }
            x.unwrap()
        }?)
    }

    // select jack by default on everything that _can_ use alsa. alsa sucks.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    {
        use rodio::cpal::traits::HostTrait;

        trace!("on \"linux\": trying to get jack");
        rodio::OutputStream::try_from_device(
            &cpal::host_from_id(
                cpal::available_hosts()
                    .into_iter()
                    .find(|x| *x == cpal::HostId::Jack)
                    .ok_or(anyhow::anyhow!("No jack host found"))?,
            )?
            .default_output_device()
            .ok_or(anyhow::anyhow!(
                "jack host found but no default output device found"
            ))?,
        )
        .map_err(|err| err.into())
    }
}
