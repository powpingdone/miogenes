use std::{io::Read, sync::{Arc, RwLock}, collections::VecDeque, time::Duration};

use log::*;
use qoaudio::QoaRodioSource;
use rodio::Source;
use uuid::Uuid;

use crate::*;

pub struct ControllingDecoder {
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