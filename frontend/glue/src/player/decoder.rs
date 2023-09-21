use crate::player::*;
use crate::*;
use crossbeam::atomic::AtomicCell;
use log::*;
use qoaudio::{QoaRodioSource, QoaDecoder};
use rodio::Source;
use std::{
    collections::{HashMap, VecDeque},
    io::{BufReader, Read, Seek},
    ops::Add,
    sync::{Arc, RwLock},
    time::Duration,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum ThreadDecoderStatus {
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
}

impl TrackInner {
    pub fn reset(&self) {
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
            std::io::SeekFrom::Start(at) => self.pos = at as usize,
            std::io::SeekFrom::End(_) => self.pos = self.internal_buf.len(),
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
    mdata: Arc<OnceLock<TrackMetadata>>
) {
    let mut server_stream = ReSeeker::new(server_stream);
    let mut decoder = QoaDecoder::new(&mut server_stream);
    loop {
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
                        let to_where = loop {
                            let mut to_where = to_where;
                            match state_change.recv_timeout(Duration::from_millis(5)) {
                                Ok(ThreadDecoderYell::Seek(at)) => to_where = at,
                                Err(crossbeam::channel::RecvTimeoutError::Timeout) => {
                                    break to_where;
                                }
                                Err(crossbeam::channel::RecvTimeoutError::Disconnected)
                                | Ok(ThreadDecoderYell::Die) => return,
                            }
                        };

                        // convert to sample time
                        let metadata = mdata.get().unwrap();

                        todo!();
                    }
                    ThreadDecoderYell::Die => unreachable!(),
                }
            }
        }
    }
}

pub(super) struct ControllingDecoder {
    // outside world interaction
    client: Arc<RwLock<MioClientState>>,
    ret_status: Arc<AtomicCell<Option<CurrentlyDecoding>>>,
    frontend_poll: std::sync::mpsc::Receiver<DecoderMsg>,
    // self status
    queue: HashMap<Uuid, TrackInner>,
    order: VecDeque<Uuid>,
    pos: usize,
    age: u64,
    active: bool,
}

impl ControllingDecoder {
    pub fn new(
        client: Arc<RwLock<MioClientState>>,
        ret_status: Arc<AtomicCell<Option<CurrentlyDecoding>>>,
        frontend_poll: std::sync::mpsc::Receiver<DecoderMsg>,
    ) -> Self {
        Self {
            client,
            ret_status,
            frontend_poll,
            queue: HashMap::new(),
            order: VecDeque::new(),
            pos: 0,
            age: 0,
            active: false,
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
        let action = match self.frontend_poll.try_recv() {
            Ok(x) => Some(x),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            // kills the audio thread, because the upper level has disconnected
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return None::<Self::Item>,
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
                    if let Some(ti) = self.get_curr_mut() {
                        let status = ti.msgs.status.load();
                        if let ThreadDecoderStatus::Dead = status {
                            todo!()
                        }
                    }

                    todo!()
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

        if !self.active {
            Some(0.0)
        } else {
            todo!()
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
