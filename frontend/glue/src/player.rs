use crate::*;
use crossbeam::channel::{Receiver, RecvTimeoutError};
use flutter_rust_bridge::StreamSink;
use qoaudio::QoaRodioSource;
use rodio::Source;
use std::{
    collections::VecDeque,
    io::Read,
    sync::{Arc, Mutex, RwLock},
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
    Stop,
    Forward,
}

#[derive(Debug)]
pub struct Player {
    pub(crate) send: crossbeam::channel::Sender<crate::player::PlayerMsg>,
}

impl Player {
    pub(crate) fn new(client: Arc<RwLock<MioClientState>>) -> Self {
        let (tx_player, rx_player) = crossbeam::channel::unbounded();

        // I don't like doing this, but i'm not joining this thread. Ignore the dropped
        // handle.
        std::thread::spawn(move || player_track_mgr(client, rx_player));
        Self { send: tx_player }
    }
}

fn player_track_mgr(client: Arc<RwLock<MioClientState>>, rx: Receiver<PlayerMsg>) {
    let mut state = PlayerState::new(client).unwrap();
    loop {
        let recv = rx.recv_deadline(Instant::now() + Duration::from_millis(50));
        match recv {
            Ok(msg) => match msg {
                PlayerMsg::SetSink(new_sink) => state.set_ui_sink(new_sink),
                PlayerMsg::Play(id) => {
                    let mut lock = state.decoder.lock().unwrap();
                    if let Some(id) = id {
                        lock.clear_self();
                        lock.set_new(id);
                        state
                            .s_handle
                            .play_raw(SharedSource {
                                i: state.decoder.clone(),
                            })
                            .unwrap();
                    }
                    lock.pause = false;
                }
                PlayerMsg::Pause => state.decoder.lock().unwrap().pause = true,
                PlayerMsg::Toggle => {
                    let mut lock = state.decoder.lock().unwrap();
                    lock.pause = !lock.pause;
                }
                PlayerMsg::Queue(id) => state.decoder.lock().unwrap().queue(id),
                PlayerMsg::Unqueue(id) => state.decoder.lock().unwrap().dequeue(id),
                PlayerMsg::Stop => state.decoder.lock().unwrap().clear_self(),
                PlayerMsg::Forward => state.decoder.lock().unwrap().forward(),
            },
            Err(err) if err == RecvTimeoutError::Disconnected => return,
            Err(err) if err == RecvTimeoutError::Timeout => (),
            _ => unreachable!(),
        }
        state.send_ui(api::PStatus { err_msg: None });
    }
}

struct SharedSource<T: Iterator> {
    pub i: Arc<Mutex<T>>,
}

impl<T> Source for SharedSource<T>
where
    T: Source,
    <T as Iterator>::Item: rodio::Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.i.lock().unwrap().current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.i.lock().unwrap().channels()
    }

    fn sample_rate(&self) -> u32 {
        self.i.lock().unwrap().sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.i.lock().unwrap().total_duration()
    }
}

impl<T> Iterator for SharedSource<T>
where
    T: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.i.lock().unwrap().next()
    }
}

struct PlayerState {
    ui_sink: Option<StreamSink<api::PStatus>>,
    _dev: rodio::OutputStream,
    s_handle: rodio::OutputStreamHandle,
    pub decoder: Arc<Mutex<ControllingDecoder>>,
}

impl PlayerState {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> anyhow::Result<Self> {
        let (_dev, s_handle) = rodio::OutputStream::try_default()?;
        Ok(Self {
            ui_sink: None,
            _dev,
            s_handle,
            decoder: Arc::new(Mutex::new(ControllingDecoder::new(client))),
        })
    }

    pub fn set_ui_sink(&mut self, ui_sink: StreamSink<api::PStatus>) {
        self.ui_sink = Some(ui_sink);
    }

    pub fn send_ui(&self, send: api::PStatus) {
        self.ui_sink.as_ref().map(|x| x.add(send));
    }
}

struct ControllingDecoder {
    true_dec: Option<qoaudio::QoaRodioSource<Box<dyn Read + Send + Sync + 'static>>>,
    pub pause: bool,
    pub vol: f32,
    client: Arc<RwLock<MioClientState>>,
    next_ids: VecDeque<Uuid>,
}

impl ControllingDecoder {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> Self {
        Self {
            true_dec: None,
            pause: false,
            vol: 1.0,
            client,
            next_ids: VecDeque::new(),
        }
    }

    pub fn set_new(&mut self, id: Uuid) {
        self.true_dec = self.set_new_inner(id).ok();
    }

    fn set_new_inner(
        &self,
        id: Uuid,
    ) -> anyhow::Result<QoaRodioSource<Box<dyn Read + Send + Sync + 'static>>> {
        Ok(qoaudio::QoaRodioSource::new(qoaudio::QoaDecoder::new(
            self.client.read().unwrap().stream(id)?,
        )?))
    }

    pub fn queue(&mut self, id: Uuid) {
        self.next_ids.push_back(id)
    }

    pub fn dequeue(&mut self, id: Uuid) {
        self.next_ids.retain(|x| *x != id)
    }

    pub fn forward(&mut self) {
        while let Some(id) = self.next_ids.pop_front() {
            self.set_new(id);
            if self.true_dec.is_none() {
                continue;
            } else {
                return;
            }
        }
    }

    pub fn clear_self(&mut self) {
        self.next_ids.clear();
        self.true_dec = None;
    }
}

impl Iterator for ControllingDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let scale = |x: i16| -> f32 { (x as f32 / i16::MAX as f32).clamp(-1.0, 1.0) * self.vol };
        if !self.pause {
            Some(0.0)
        } else if let Some(ref mut dec) = self.true_dec {
            let x = dec.next().map(scale);
            if x.is_none() {
                loop {
                    if let Some(id) = self.next_ids.pop_front() {
                        self.set_new(id);
                        if self.true_dec.is_none() {
                            continue;
                        }
                        return self.next();
                    } else {
                        self.true_dec = None;
                        return Some(0.0);
                    }
                }
            }
            x
        } else {
            Some(0.0)
        }
    }
}

impl Source for ControllingDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        self.true_dec.as_ref().and_then(|x| x.current_frame_len())
    }

    fn channels(&self) -> u16 {
        self.true_dec
            .as_ref()
            .map(|x| x.channels())
            .unwrap_or_default()
    }

    fn sample_rate(&self) -> u32 {
        self.true_dec
            .as_ref()
            .map(|x| x.sample_rate())
            .unwrap_or_default()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
