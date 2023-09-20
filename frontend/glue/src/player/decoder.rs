use crate::player::*;
use crate::*;
use crossbeam::atomic::AtomicCell;
use log::*;
use qoaudio::QoaRodioSource;
use ringbuffer::RingBuffer;
use rodio::Source;
use std::{
    cell::UnsafeCell,
    collections::{HashMap, VecDeque},
    sync::{atomic::AtomicBool, mpsc::TryRecvError, Arc, RwLock},
    time::Duration,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum ThreadDecoderStatus {
    Buffering,
    Loading,
    Ready,
    Dead,
}

enum ThreadDecoderYell {
    Die,
    Seek(Duration),
}

struct TrackInner {
    // thread interaction
    pub buf: Arc<crossbeam::channel::Receiver<f32>>,
    pub status: Arc<AtomicCell<ThreadDecoderStatus>>,
    pub back_to_decoder: crossbeam::channel::Sender<ThreadDecoderYell>,
    // metadata
    pub len: Duration,
    pub channels: u16,
    pub sample_rate: u32,
    pub age: u64,
}

impl TrackInner {
    pub fn reset(&self) {
        // send back to beginning
        if let Err(res) = self
            .back_to_decoder
            .send(ThreadDecoderYell::Seek(Duration::from_secs(0)))
        {
            debug!("error when yelling at decoder for stopping: {:?}", res);
        }

        // clear queue
        for _ in 0..self.buf.capacity().unwrap() {
            drop(self.buf.try_recv());
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
            Err(TryRecvError::Empty) => None,
            // kills the audio thread, because the upper level has disconnected
            Err(TryRecvError::Disconnected) => return None::<Self::Item>,
        };

        if let Some(action) = action {
            match action {
                DecoderMsg::SeekAbs(dur) => {
                    if let Some(tinner) = self.get_curr() {
                        if let Err(res) = tinner.back_to_decoder.send(ThreadDecoderYell::Seek(dur))
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
                        let status = ti.status.load();
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
        self.get_curr().map(|x| x.channels).unwrap_or(1)
    }

    fn sample_rate(&self) -> u32 {
        self.get_curr().map(|x| x.sample_rate).unwrap_or(96000)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
