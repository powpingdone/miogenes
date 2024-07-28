use gstreamer::{self as gst, prelude::GstBinExtManual};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

pub struct NoQueueLeft;

// frontend interaction with audio player
pub struct AudioPlayer {
    order: VecDeque<Uuid>,
    now_playing: usize,
    decoder_pipelines: HashMap<Uuid, gst::Pipeline>,
    limit: usize,
}

impl AudioPlayer {
    pub fn new(limit: usize) -> Result<Self, anyhow::Error> {
        gst::init()?;
        Ok(Self {
            order: VecDeque::with_capacity(limit),
            now_playing: 0,
            decoder_pipelines: HashMap::new(),
            limit,
        })
    }

    pub fn queue(&mut self, id: Uuid) -> Result<Option<Uuid>, NoQueueLeft> {
        let new_id = if self.limit >= self.order.len() {
            if self.now_playing == 0 {
                return Err(NoQueueLeft);
            }
            self.order.pop_front()
        } else {
            None
        };
        new_id.and_then(|x| self.decoder_pipelines.remove(&x));
        self.add_pipeline(id);
        return Ok(new_id);
    }

    fn add_pipeline(&mut self, id: Uuid) {
        todo!()
    }
}

fn make_sink() -> anyhow::Result<gst::Pipeline> {
    let audio_sink = gst::Pipeline::builder().name("audio_sink").build();
    let conv = gst::ElementFactory::make("audioconvert").build()?;
    let samp = gst::ElementFactory::make("audioresample").build()?;
    let sink = gst::ElementFactory::make("autoaudiosink").build()?;
    audio_sink.add_many([&conv, &samp, &sink])?;
    gst::Element::link_many([&conv, &samp, &sink])?;
    Ok(audio_sink)
}
