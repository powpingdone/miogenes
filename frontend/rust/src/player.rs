use gstreamer::{self as gst, prelude::*, SampleRef};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

use crate::{GlueError, GlueResult};

// frontend interaction with audio player
pub struct AudioPlayer {
    order: VecDeque<Uuid>,
    now_playing: usize,
    limit: usize,
    pipeline: gst::Pipeline,
    souphttp: gst::Element,
}

impl AudioPlayer {
    pub fn new(limit: usize) -> GlueResult<Self> {
        // setup pipeline
        let pipeline = gst::Pipeline::new();
        let souphttp = gst::ElementFactory::make("souphttpsrc").build()?;
        let decodebin = gst::ElementFactory::make("decodebin3").build()?;
        let audioconv = gst::ElementFactory::make("audioconvert").build()?;
        let audiorsam = gst::ElementFactory::make("audioresample").build()?;
        let audiosink = gst::ElementFactory::make("autoaudiosink").build()?;
        pipeline.add_many([&souphttp, &decodebin, &audioconv, &audiorsam, &audiosink])?;
        gst::Element::link_many([&souphttp, &decodebin, &audioconv, &audiorsam, &audiosink])?;
        souphttp.set_property("location", None::<&str>);
        souphttp.set_property("extra-headers", None::<&gst::Structure>);
        pipeline.set_state(gst::State::Null)?;
        // return
        Ok(Self {
            order: VecDeque::with_capacity(limit),
            now_playing: 0,
            limit,
            pipeline,
            souphttp,
        })
    }

    pub fn queue(&mut self, id: Uuid) -> GlueResult<Option<Uuid>> {
        let old_id = if self.limit >= self.order.len() {
            if self.now_playing == 0 {
                return Err(GlueError::NoSpaceLeftInQueue);
            }
            self.now_playing -= 1;
            self.order.pop_front()
        } else {
            None
        };
        if self.pipeline.current_state() == gst::State::Null {
            self.pipepline_play();
        }
        return Ok(old_id);
    }

    pub fn reset(&mut self) {
        self.order.clear();
        self.now_playing = 0;
        self.souphttp.set_property("location", None::<&str>);
        self.souphttp.set_property("extra-headers", None::<&gst::Structure>);
        self.pipeline.set_state(gst::State::Null).unwrap();
    }

    fn pipepline_play(&self) {
        todo!();
        // set location, extra-headers, and pipeline state
    }
    
}
