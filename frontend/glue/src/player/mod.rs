use std::time::Duration;
use uuid::Uuid;

mod audio_dev;
mod decoder;

pub use audio_dev::*;

#[derive(Default)]
pub struct TrackDecoderMetaData {
    pub id: Uuid,
}

#[derive(Default)]
pub struct CurrentlyDecoding {
    pub tracks: Vec<TrackDecoderMetaData>,
    pub curr: Uuid,
    pub at: Duration,
    pub len: Duration,
}

pub enum DecoderMsg {
    SeekAbs(Duration),
    Enqueue(Uuid),
    Play,
    Reset,
    Pause,
    Stop,
    Next,
    Previous,
}
