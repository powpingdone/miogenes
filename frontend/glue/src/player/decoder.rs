use crate::player::*;
use crate::*;
use crossbeam::atomic::AtomicCell;
use log::*;
use qoaudio::QoaRodioSource;
use rodio::Source;
use std::{
    collections::HashMap,
    sync::{mpsc::TryRecvError, Arc, RwLock, atomic::AtomicBool},
    time::Duration, cell::UnsafeCell,
};
use uuid::Uuid;

#[derive(Clone, Copy)]
enum ThreadDecoderStatus {
    NoThread,
    Buffering,
    Loading,
    Ready,
}


struct TrackInner {
    pub buf: SharedBuffer,
    pub len: Duration,
    pub status: Arc<AtomicCell<ThreadDecoderStatus>>,
    pub channels: u16,
    pub sample_rate: u32,
    pub age: u64,
}

pub(super) struct ControllingDecoder {
    // outside world interaction
    client: Arc<RwLock<MioClientState>>,
    ret_status: Arc<AtomicCell<Option<CurrentlyDecoding>>>,
    frontend_poll: std::sync::mpsc::Receiver<DecoderMsg>,
    // self status
    curr: Uuid,
    queue: HashMap<Uuid, TrackInner>,
    pos: u32,
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
            curr: Uuid::nil(),
            pos: 0,
            age: 0,
            active: false,
        }
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
        self.queue.get(&self.curr).map(|x| x.channels).unwrap_or(1)
    }

    fn sample_rate(&self) -> u32 {
        self.queue
            .get(&self.curr)
            .map(|x| x.sample_rate)
            .unwrap_or(96000)
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
