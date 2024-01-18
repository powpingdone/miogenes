use crate::player::*;
use crate::*;
use crossbeam::atomic::AtomicCell;
use log::*;
use qoaudio::QoaDecoder;
use rodio::Source;
use tokio::sync::RwLock;
use std::{
    collections::{HashMap, VecDeque},
    io::{Read, Seek},
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
enum ThreadDecoderStatus {
    WaitingForThread,
    Decoding,
    Loading,
    Ready,
    Dead,
}

#[derive(PartialEq)]
enum ThreadDecoderYell {
    Die,
    Seek(Duration),
}

/// thread msg parts
struct ThreadMsgs {
    pub buf: crossbeam::channel::Receiver<f32>,
    pub status: Arc<AtomicCell<ThreadDecoderStatus>>,
    pub back_to_decoder: crossbeam::channel::Sender<ThreadDecoderYell>,
    pub at: Arc<AtomicCell<f64>>,
}

/// Track metadata
struct TrackMetadata {
    pub len: Duration,
    pub channels: u16,
    pub sample_rate: u32,
}

/// Thread interaction structure. Talks to the thread as an abstraction.
struct TrackInner {
    pub msgs: ThreadMsgs,
    pub mdata: Arc<OnceLock<TrackMetadata>>,
    pub age: u64,
    _t_handle: JoinHandle<()>,
}

impl TrackInner {
    fn new(download: Box<dyn Read + Send + Sync + 'static>, age: u64) -> Self {
        // thread comms
        let (buf_tx, buf_rx) = crossbeam::channel::bounded(16384);
        let status = Arc::new(AtomicCell::new(ThreadDecoderStatus::WaitingForThread));
        let (decoder_tx, decoder_rx) = crossbeam::channel::unbounded();
        let at = Arc::new(AtomicCell::new(0.0));

        // other
        let mdata = Arc::new(OnceLock::new());
        let _t_handle = std::thread::spawn({
            let status = status.clone();
            let mdata = mdata.clone();
            let at = at.clone();
            move || decoder_thread(buf_tx, status, decoder_rx, download, mdata, at)
        });
        Self {
            msgs: ThreadMsgs {
                buf: buf_rx,
                status,
                back_to_decoder: decoder_tx,
                at,
            },
            mdata,
            age,
            _t_handle,
        }
    }

    fn reset(&self) {
        // send back to beginning
        if let Err(res) = self
            .msgs
            .back_to_decoder
            .send(ThreadDecoderYell::Seek(Duration::from_secs(0)))
        {
            debug!("error when yelling at decoder for stopping: {:?}", res);
        }

        // clear queue
        for _ in 0..self.msgs.buf.capacity().unwrap() {
            let _ = self.msgs.buf.try_recv();
        }
    }
}

impl Drop for TrackInner {
    fn drop(&mut self) {
        drop(self.msgs.back_to_decoder.send(ThreadDecoderYell::Die));
    }
}

/// wraps the ureq reader and gives seeking capabilities
struct ReSeeker {
    reader: Box<dyn Read + Send + Sync + 'static>,
    internal_buf: Vec<u8>,
    pos: usize,
}

impl ReSeeker {
    fn new(reader: Box<dyn Read + Send + Sync + 'static>) -> Self {
        Self {
            reader,
            internal_buf: vec![],
            pos: 0,
        }
    }
}

impl Read for ReSeeker {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut read = 0;
        let buf_len = buf.len();
        let mut buf_iter = buf.iter_mut();

        // read internal_buf
        while self.internal_buf.get(self.pos).is_some() {
            if let Some(byte) = buf_iter.next() {
                *byte = self.internal_buf[self.pos];
            } else {
                return Ok(read);
            }
            self.pos += 1;
            read += 1;
        }

        // read from internet
        let mut remaining = vec![0u8; buf_len - read];
        let read = self.reader.read(remaining.as_mut_slice())? + read;

        // put into internal_buf
        self.internal_buf.extend_from_slice(&remaining);
        self.pos = self.internal_buf.len();

        // copy into output
        for (byte, copy) in buf_iter.zip(remaining.into_iter()) {
            *byte = copy;
        }
        Ok(read)
    }
}

impl Seek for ReSeeker {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Start(at) => {
                self.pos = (at as usize).clamp(0, self.internal_buf.len())
            }
            std::io::SeekFrom::End(at) => {
                self.pos = {
                    let x = self.internal_buf.len();
                    if at.is_negative() {
                        x.saturating_sub(at.abs() as usize)
                    } else {
                        x.saturating_add(at.abs() as usize)
                    }
                }
                .clamp(0, self.internal_buf.len())
            }
            std::io::SeekFrom::Current(append) => {
                self.pos = {
                    let abs = append.abs() as usize;
                    if append.is_negative() {
                        self.pos.saturating_sub(abs)
                    } else {
                        self.pos.saturating_add(abs)
                    }
                }
                .clamp(0, self.internal_buf.len())
            }
        }
        Ok(self.pos as u64)
    }
}

fn decoder_thread(
    waveform: crossbeam::channel::Sender<f32>,
    status: Arc<AtomicCell<ThreadDecoderStatus>>,
    state_change: crossbeam::channel::Receiver<ThreadDecoderYell>,
    server_stream: Box<dyn Read + Send + Sync + 'static>,
    mdata: Arc<OnceLock<TrackMetadata>>,
    at: Arc<AtomicCell<f64>>,
) {
    status.store(ThreadDecoderStatus::Ready);
    decoder_thread_inner(
        waveform,
        status.clone(),
        state_change,
        server_stream,
        mdata,
        at,
    );
    status.store(ThreadDecoderStatus::Dead);
}

fn decoder_thread_inner(
    waveform: crossbeam::channel::Sender<f32>,
    status: Arc<AtomicCell<ThreadDecoderStatus>>,
    state_change: crossbeam::channel::Receiver<ThreadDecoderYell>,
    server_stream: Box<dyn Read + Send + Sync + 'static>,
    mdata: Arc<OnceLock<TrackMetadata>>,
    at: Arc<AtomicCell<f64>>,
) {
    // setup decoder
    let mut server_stream = ReSeeker::new(server_stream);
    let mut decoder = match QoaDecoder::new(&mut server_stream) {
        Ok(x) => x,
        Err(err) => {
            error!("decoder died: {err}");
            return;
        }
    };
    let (channels, sample_rate, samples) = match decoder.mode() {
        qoaudio::ProcessingMode::FixedSamples {
            channels,
            sample_rate,
            samples,
        } => (channels, sample_rate, samples),
        qoaudio::ProcessingMode::Streaming => {
            error!("the decoded file is a streaming file");
            return;
        }
    };
    drop(mdata.set(TrackMetadata {
        len: Duration::from_secs_f64(*samples as f64 / *sample_rate as f64),
        channels: *channels as u16,
        sample_rate: *sample_rate,
    }));
    let mut next_samp: Option<f32> = None;
    let mut pos = 0usize;

    // decoder loop
    loop {
        // handle state change
        match state_change.try_recv() {
            Err(crossbeam::channel::TryRecvError::Disconnected) | Ok(ThreadDecoderYell::Die) => {
                return
            }
            Err(crossbeam::channel::TryRecvError::Empty) => (),
            Ok(action) => {
                match action {
                    ThreadDecoderYell::Seek(to_where) => {
                        status.store(ThreadDecoderStatus::Decoding);

                        // debounce
                        let mut to_where_inner = to_where;
                        let to_where = loop {
                            match state_change.recv_timeout(Duration::from_millis(5)) {
                                Ok(ThreadDecoderYell::Seek(at)) => to_where_inner = at,
                                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                                    break to_where_inner;
                                }
                                Err(crossbeam::channel::RecvTimeoutError::Disconnected)
                                | Ok(ThreadDecoderYell::Die) => return,
                            }
                        };

                        // get samples needed
                        let metadata = mdata.get().unwrap();
                        let how_many_dur = metadata.len - to_where;
                        let how_many = (how_many_dur.as_secs_f64() * metadata.sample_rate as f64)
                            .floor() as usize;

                        // recreate decoder
                        server_stream.seek(std::io::SeekFrom::Start(0)).unwrap();
                        let mut new_dec = QoaDecoder::new(&mut server_stream).unwrap();
                        (0..how_many).for_each(|_| drop(new_dec.next()));
                        decoder = new_dec;
                        pos = how_many;
                    }
                    ThreadDecoderYell::Die => unreachable!(),
                }
            }
        }

        // send over new waveform
        if next_samp.is_none() {
            status.store(ThreadDecoderStatus::Loading);
            next_samp = match decoder.next() {
                None => return,
                Some(Err(err)) => {
                    error!("decoding error: {err}");
                    return;
                }
                Some(Ok(dec)) => match dec {
                    qoaudio::QoaItem::Sample(x) => {
                        Some((x as f32 / i16::MAX as f32).clamp(-1.0, 1.0))
                    }
                    qoaudio::QoaItem::FrameHeader(_) => continue,
                },
            };
        }
        match waveform.send_timeout(next_samp.unwrap(), Duration::from_millis(5)) {
            Ok(_) => {
                next_samp = None;
                pos += 1;
                at.store(pos as f64 / mdata.get().unwrap().sample_rate as f64);
            }
            Err(crossbeam::channel::SendTimeoutError::Timeout(_)) => {
                status.store(ThreadDecoderStatus::Ready);
                continue;
            }
            Err(crossbeam::channel::SendTimeoutError::Disconnected(_)) => return,
        }
    }
}

pub(super) struct ControllingDecoder {
    // outside world interaction
    client: Arc<RwLock<MioClientState>>,
    ret_status: tokio::sync::watch::Sender<CurrentlyDecoding>,
    frontend_poll: crossbeam::channel::Receiver<DecoderMsg>,
    time_since_last_msg: std::time::Instant,
    // self status
    queue: HashMap<Uuid, TrackInner>,
    order: VecDeque<Uuid>,
    pos: usize,
    age: u64,
    active: bool,
    time_since_last_cleanup: std::time::Instant,
}

impl ControllingDecoder {
    pub fn new(
        client: Arc<RwLock<MioClientState>>,
        ret_status: tokio::sync::watch::Sender<CurrentlyDecoding>,
        frontend_poll: crossbeam::channel::Receiver<DecoderMsg>,
    ) -> Self {
        Self {
            client,
            ret_status,
            frontend_poll,
            time_since_last_msg: Instant::now(),
            queue: HashMap::new(),
            order: VecDeque::new(),
            pos: 0,
            age: 0,
            active: false,
            time_since_last_cleanup: Instant::now(),
        }
    }

    fn get_curr(&self) -> Option<&TrackInner> {
        self.queue
            .get(&self.order.get(self.pos).unwrap_or(&Uuid::nil()))
    }

    fn get_curr_mut(&mut self) -> Option<&mut TrackInner> {
        self.queue
            .get_mut(&self.order.get(self.pos).unwrap_or(&Uuid::nil()))
    }
}

impl Iterator for ControllingDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // handle outside world msg
        let action = match self.frontend_poll.try_recv() {
            Ok(x) => Some(x),
            Err(crossbeam::channel::TryRecvError::Empty) => None,
            // kills the audio thread, because the upper level has disconnected
            Err(crossbeam::channel::TryRecvError::Disconnected) => return None::<Self::Item>,
        };
        if let Some(action) = action {
            match action {
                DecoderMsg::SeekAbs(dur) => {
                    if let Some(tinner) = self.get_curr() {
                        if let Err(res) = tinner
                            .msgs
                            .back_to_decoder
                            .send(ThreadDecoderYell::Seek(dur))
                        {
                            debug!("error when yelling at decoder: {:?}", res);
                        }
                    }
                }
                DecoderMsg::Enqueue(id) => self.order.push_front(id),
                DecoderMsg::Play => self.active = true,
                DecoderMsg::Pause => self.active = false,
                DecoderMsg::Stop => {
                    if let Some(ti) = self.get_curr() {
                        ti.reset();
                    }
                    self.active = false;
                }
                DecoderMsg::Next | DecoderMsg::Previous => {
                    if let Some(ti) = self.get_curr() {
                        ti.reset();

                        // change decoder
                        if let DecoderMsg::Next = action {
                            self.pos = self.pos.saturating_add(1);
                        } else {
                            self.pos = self.pos.saturating_sub(1);
                        }
                    }

                    // reset if dead
                    let mut dead = false;
                    if let Some(ti) = self.get_curr() {
                        let status = ti.msgs.status.load();
                        if let ThreadDecoderStatus::Dead = status {
                            dead = true;
                        }
                    }
                    if dead {
                        // minor work around for borrowing issues
                        self.age += 1;
                    }
                }
                DecoderMsg::Reset => {
                    self.queue.clear();
                    self.order.clear();
                    self.pos = 0;
                    self.age = 0;
                    self.active = false;
                }
            }
        }

        // periodic status update
        let instant_now = Instant::now();
        if self.time_since_last_msg.duration_since(instant_now) > Duration::from_millis(50) {
            self.time_since_last_msg = instant_now;
            let ret = self.ret_status.send({
                match self.get_curr() {
                    Some(track) => {
                        let mdata = track.mdata.get();
                        if let Some(mdata) = mdata {
                            // set status
                            CurrentlyDecoding {
                                len: mdata.len,
                                at: Duration::from_secs_f64(track.msgs.at.load()),
                                curr: self.order[self.pos],
                                tracks: {
                                    self.order
                                        .iter()
                                        .copied()
                                        .map(|x| TrackDecoderMetaData { id: x })
                                        .collect()
                                },
                            }
                        } else {
                            CurrentlyDecoding::default()
                        }
                    }
                    None => CurrentlyDecoding::default(),
                }
            });
            if ret.is_err() {
                // apparently, now is when the watcher got dropped. die.
                return None::<Self::Item>;
            }
        }

        // cleanup
        if self.time_since_last_cleanup.duration_since(Instant::now()) > Duration::from_secs(1) {
            loop {
                let Some(x) = self.order.get(0).cloned() else {
                    break;
                };
                let Some(curr) = self.get_curr() else {
                    break;
                };
                if curr.age - self.queue[&x].age < 25 {
                    break;
                }
                if self.queue[&x].msgs.status.load() == ThreadDecoderStatus::Dead {
                    self.order.pop_front();
                    self.queue.remove(&x);
                }
            }
        }

        // give back current sample
        if !self.active || self.get_curr().is_none() {
            Some(0.0)
        } else {
            let x = self.get_curr().and_then(|x| x.msgs.buf.try_recv().ok());
            if x.is_none()
                && self.get_curr().unwrap().msgs.status.load() == ThreadDecoderStatus::Dead
            {
                // goto next track
                self.pos += 1;
                self.next()
            } else {
                x
            }
        }
    }
}

impl Source for ControllingDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        Some(1024)
    }

    fn channels(&self) -> u16 {
        self.get_curr()
            .and_then(|x| x.mdata.get().map(|x| x.channels))
            .unwrap_or(1)
    }

    fn sample_rate(&self) -> u32 {
        self.get_curr()
            .and_then(|x| x.mdata.get().map(|x| x.sample_rate))
            .unwrap_or(96000)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
