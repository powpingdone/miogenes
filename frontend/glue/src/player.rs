use crate::*;
use crossbeam::channel::{Receiver, RecvTimeoutError};
use flutter_rust_bridge::StreamSink;
use log::*;
use parking_lot::Mutex;
use qoaudio::QoaRodioSource;
use rodio::Source;
use std::{
    collections::VecDeque,
    fmt::Debug,
    io::Read,
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
    Volume(f32),
    Stop,
    Forward,
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
            Self::Volume(arg0) => f.debug_tuple("Volume").field(arg0).finish(),
            Self::Stop => write!(f, "Stop"),
            Self::Forward => write!(f, "Forward"),
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

        // I don't like doing this, but i'm not joining this thread. Ignore the dropped
        // handle.
        std::thread::Builder::new()
            .name("MioPlayerT".to_owned())
            .spawn(move || player_track_mgr(client, rx_player))
            .unwrap();
        Self { tx: tx_player }
    }
}

fn player_track_mgr(client: Arc<RwLock<MioClientState>>, rx: Receiver<PlayerMsg>) {
    trace!("opening track manager");
    let mut state = match PlayerState::new(client) {
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
                            volume: 0.0,
                            paused: true,
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
                    let mut lock = state.decoder.lock();
                    if let Some(id) = id {
                        lock.clear_self();
                        lock.set_new(id);
                    }
                    lock.pause = false;
                    lock.dec_kickover();
                }
                PlayerMsg::Pause => state.decoder.lock().pause = true,
                PlayerMsg::Toggle => {
                    let mut lock = state.decoder.lock();
                    lock.pause = !lock.pause;
                }
                PlayerMsg::Queue(id) => state.decoder.lock().queue(id),
                PlayerMsg::Unqueue(id) => state.decoder.lock().dequeue(id),
                PlayerMsg::Stop => state.decoder.lock().clear_self(),
                PlayerMsg::Forward => state.decoder.lock().forward(),
                PlayerMsg::Volume(vol) => state.decoder.lock().vol = vol,
            },
            Err(err) if err == RecvTimeoutError::Disconnected => return,
            Err(err) if err == RecvTimeoutError::Timeout => (),
            _ => unreachable!(),
        }

        // yes double locking is very much shitty and suboptimal, but PlayerMsg::SetSink
        // forced my hand. why do we not have partial borrows yet
        let lock = state.decoder.lock();
        state.send_ui(api::PStatus {
            err_msg: _err_msg,
            queue: lock.copy_queue(),
            volume: lock.vol,
            paused: lock.pause,
        });
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
        self.i.lock().current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.i.lock().channels()
    }

    fn sample_rate(&self) -> u32 {
        self.i.lock().sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.i.lock().total_duration()
    }
}

impl<T> Iterator for SharedSource<T>
where
    T: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.i.lock().next()
    }
}

struct PlayerState {
    ui_sink: Option<StreamSink<api::PStatus>>,
    _dev: rodio::OutputStream,
    _s_handle: rodio::OutputStreamHandle,
    _s_thread: std::thread::JoinHandle<()>,
    pub decoder: Arc<Mutex<ControllingDecoder>>,
}

impl PlayerState {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> anyhow::Result<Self> {
        trace!("acqiring dev");
        let (_dev, s_handle) = find_dev()?;
        trace!("setting up decoder");
        let decoder = Arc::new(Mutex::new(ControllingDecoder::new(client)));
        Ok(Self {
            ui_sink: None,
            _dev,
            _s_thread: std::thread::spawn({
                let decoder = decoder.clone();
                let s_handle = s_handle.clone();
                move || {
                    trace!("spinning s_thread");
                    s_handle.play_raw(SharedSource { i: decoder }).unwrap();
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
        use cpal::traits::HostTrait;

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

struct ControllingDecoder {
    true_dec: Option<qoaudio::QoaRodioSource<Box<dyn Read + Send + Sync + 'static>>>,
    pub pause: bool,
    pub vol: f32,
    playing_id: Option<Uuid>,
    client: Arc<RwLock<MioClientState>>,
    next_ids: VecDeque<Uuid>,
}

impl ControllingDecoder {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> Self {
        Self {
            true_dec: None,
            pause: false,
            vol: 1.0,
            playing_id: None,
            client,
            next_ids: VecDeque::new(),
        }
    }

    pub fn dec_kickover(&mut self) -> bool {
        if self.true_dec.is_none() && self.playing_id.is_none() && !self.next_ids.is_empty() {
            // needed to be kicked
            debug!("kicking over");
            self.forward();
            true
        } else {
            // is ready
            trace!(
                "no kickover required: {} && {} && {}",
                self.true_dec.is_none(),
                self.playing_id.is_none(),
                !self.next_ids.is_empty()
            );
            false
        }
    }

    pub fn set_new(&mut self, id: Uuid) {
        let dec = self.set_new_inner(id);
        if let Err(ref err) = dec {
            warn!("error setting up new decoder: {err}");
        }
        self.true_dec = dec.ok();
        if self.true_dec.is_some() {
            self.playing_id = Some(id);
        }
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
        trace!("queueing {id}");
        self.next_ids.push_back(id)
    }

    pub fn dequeue(&mut self, id: Uuid) {
        trace!("dequeueing {id}");
        self.next_ids.retain(|x| *x != id)
    }

    pub fn forward(&mut self) {
        loop {
            if let Some(id) = self.next_ids.pop_front() {
                debug!("next track is {id}, setting");
                self.set_new(id);
                if self.true_dec.is_none() {
                    continue;
                } else {
                    return;
                }
            } else {
                debug!("no more tracks to set");
                self.playing_id = None;
                self.true_dec = None;
                return;
            }
        }
    }

    pub fn clear_self(&mut self) {
        trace!("cleaning self");
        self.next_ids.clear();
        self.true_dec = None;
        self.playing_id = None;
        self.pause = true;
    }

    pub fn copy_queue(&self) -> Vec<Uuid> {
        let mut ret = if self.playing_id.is_some() {
            vec![self.playing_id.unwrap()]
        } else {
            vec![]
        };
        ret.extend(self.next_ids.iter().copied());
        ret
    }
}

impl Iterator for ControllingDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let scale = |x: i16| -> f32 { (x as f32 / i16::MAX as f32).clamp(-1.0, 1.0) * self.vol };
        if self.pause {
            Some(0.0)
        } else if let Some(ref mut dec) = self.true_dec {
            let sample = dec.next().map(scale);

            // if finished
            if sample.is_none() {
                // there is no need to notify player_track_mgr because it will poll and then pick
                // this up, once it acquires the lock
                if self.next_ids.is_empty() {
                    self.clear_self();
                } else {
                    self.forward();
                }
                return self.next();
            }
            sample
        } else {
            Some(0.0)
        }
    }
}

impl Source for ControllingDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        Some(1024)
    }

    fn channels(&self) -> u16 {
        self.true_dec.as_ref().map(|x| x.channels()).unwrap_or(1)
    }

    fn sample_rate(&self) -> u32 {
        self.true_dec
            .as_ref()
            .map(|x| x.sample_rate())
            .unwrap_or(96000)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
