use std::time::Duration;
use uuid::Uuid;

mod audio_dev;
mod decoder;

pub use audio_dev::*;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct TrackDecoderMetaData {
    pub id: Uuid,
}

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct CurrentlyDecoding {
    pub curr: Uuid,
    pub at: Duration,
    pub len: Duration,
    pub tracks: Vec<TrackDecoderMetaData>,
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
