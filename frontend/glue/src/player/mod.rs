use crate::api;
use std::time::Duration;
use uuid::Uuid;

mod audio_dev;
mod decoder;

pub use audio_dev::*;

pub(self) struct TrackDecoderMetaData {
    pub id: Uuid,
    pub status: api::DecoderStatus,
}

pub(self) struct CurrentlyDecoding {
    pub tracks: Vec<TrackDecoderMetaData>,
    pub curr: Uuid,
    pub at: Duration,
    pub len: Duration,
}

pub(self) enum DecoderMsg {
    SeekAbs(Duration),
    Enqueue(Uuid),
    Play,
    Reset,
    Pause,
    Stop,
    Next,
    Previous,
}
