use gstreamer::{self as gst, prelude::GstBinExtManual};
use std::collections::HashMap;
use uuid::Uuid;

// frontend interaction with audio player
pub struct AudioPlayer {
    order: Vec<Uuid>,
    now_playing: usize,
    decoder_pipelines: HashMap<Uuid, gst::Pipeline>,
    audio_sink: gst::Pipeline,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, anyhow::Error> {
        gst::init()?;
        let audio_sink = gst::Pipeline::builder().name("audio_sink").build();
        let conv = gst::ElementFactory::make("audioconvert").build()?;
        let samp = gst::ElementFactory::make("audioresample").build()?;
        let sink = gst::ElementFactory::make("autoaudiosink").build()?;
        audio_sink.add_many([&conv, &samp, &sink])?;
        gst::Element::link_many([&conv, &samp, &sink])?;
        Ok(Self {
            order: vec![],
            now_playing: 0,
            decoder_pipelines: HashMap::new(),
            audio_sink,
        })
    }
}
