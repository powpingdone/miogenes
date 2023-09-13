use crate::player::decoder::ControllingDecoder;
use crate::*;
use crossbeam::channel::{Receiver, RecvTimeoutError};
use flutter_rust_bridge::StreamSink;
use log::*;
use parking_lot::Mutex;
use rodio::Source;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) enum PlayerMsg {
    SetSink(StreamSink<api::PStatus>),
    Play(Option<Uuid>),
    Pause,
    Toggle,
    Queue(Uuid),
    Unqueue(Uuid),
    Stop,
    Forward,
    Backward,
    Seek(Duration),
}

impl std::fmt::Debug for PlayerMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SetSink(_) => f.debug_tuple("SetSink").finish(),
            Self::Play(arg0) => f.debug_tuple("Play").field(arg0).finish(),
            Self::Pause => write!(f, "Pause"),
            Self::Toggle => write!(f, "Toggle"),
            Self::Queue(arg0) => f.debug_tuple("Queue").field(arg0).finish(),
            Self::Unqueue(arg0) => f.debug_tuple("Unqueue").field(arg0).finish(),
            Self::Stop => write!(f, "Stop"),
            Self::Forward => write!(f, "Forward"),
            Self::Backward => write!(f, "Backward"),
            Self::Seek(dur) => f.debug_tuple("Seek").field(dur).finish(),
        }
    }
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
    let mut state = match PlayerState::new(client.clone()) {
        Ok(x) => x,
        Err(err) => loop {
            let mut sink = None;
            let recv = rx.recv();
            match recv {
                Ok(x) => {
                    if let PlayerMsg::SetSink(sink_now) = x {
                        sink = Some(sink_now);
                    }
                    sink.as_ref().map(|sink| {
                        sink.add(api::PStatus {
                            err_msg: Some(err.to_string()),
                            queue: Vec::new(),
                            status: None,
                            curr_playing: None,
                            playback_pos: 0,
                        })
                    });
                }
                Err(_) => return,
            }
        },
    };
    trace!("entering event loop");
    loop {
        let mut _err_msg = None;
        let recv = rx.recv_deadline(Instant::now() + Duration::from_millis(50));
        match recv {
            Ok(msg) => match msg {
                PlayerMsg::SetSink(new_sink) => state.set_ui_sink(new_sink),
                PlayerMsg::Play(id) => {
                    todo!()
                }
                PlayerMsg::Pause => todo!(),
                PlayerMsg::Toggle => {
                    todo!()
                }
                PlayerMsg::Queue(id) => todo!(),
                PlayerMsg::Unqueue(id) => todo!(),
                PlayerMsg::Stop => todo!(),
                PlayerMsg::Forward => todo!(),
                PlayerMsg::Backward => todo!(),
                PlayerMsg::Seek(_dur) => todo!(),
            },
            Err(err) if err == RecvTimeoutError::Disconnected => return,
            Err(err) if err == RecvTimeoutError::Timeout => (),
            _ => unreachable!(),
        }

        // yes double locking is very much shitty and suboptimal, but PlayerMsg::SetSink
        // forced my hand.
        let queue: Vec<api::MediaStatus> = todo!();
        let status: Option<api::DecoderStatus> = todo!();
        let curr_playing = todo!();
        let playback_pos = todo!();

        // add to radio queue
        if queue.len() < 50 && status.is_some() {
            let next = client
                .read()
                .unwrap()
                .get_closest(
                    queue[0].id,
                    queue.clone().into_iter().map(|x| x.id).collect(),
                )
                .unwrap()
                .id;

            // next iteration will pickup the new id in the queue
            todo!()
        }
        state.send_ui(api::PStatus {
            err_msg: _err_msg,
            queue,
            status,
            curr_playing,
            playback_pos,
        });
    }
}

struct PlayerState {
    ui_sink: Option<StreamSink<api::PStatus>>,
    _dev: rodio::OutputStream,
    _s_handle: rodio::OutputStreamHandle,
    _s_thread: std::thread::JoinHandle<()>,
    pub decoder: (),
}

impl PlayerState {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> anyhow::Result<Self> {
        trace!("acqiring dev");
        let (_dev, s_handle) = find_dev()?;
        trace!("setting up decoder");
        let decoder: () = todo!();
        Ok(Self {
            ui_sink: None,
            _dev,
            _s_thread: std::thread::spawn({
                let decoder = decoder.clone();
                let s_handle = s_handle.clone();
                move || {
                    trace!("spinning s_thread");

                    // s_handle.play_raw( "source" ).unwrap();
                    todo!()
                }
            }),
            _s_handle: s_handle,
            decoder,
        })
    }

    pub fn set_ui_sink(&mut self, ui_sink: StreamSink<api::PStatus>) {
        self.ui_sink = Some(ui_sink);
    }

    pub fn send_ui(&self, send: api::PStatus) {
        self.ui_sink.as_ref().map(|x| x.add(send));
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
