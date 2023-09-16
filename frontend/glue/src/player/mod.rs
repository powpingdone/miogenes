use crate::api;
use std::time::Duration;
use uuid::Uuid;

mod audio_dev;
mod decoder;

pub use audio_dev::*;

pub(self) struct TrackDecoderMetaData {
    pub id: Uuid,
    pub buffered: Duration,
    pub duration: Duration,
    pub status: api::DecoderStatus,
}

pub(self) struct CurrentlyDecoding {
    pub tracks: Vec<TrackDecoderMetaData>,
    pub curr: Uuid,
    pub at: Duration,
}

pub(self) enum DecoderMsg {
    SeekAbs(Duration),
    Load(Uuid),
    Play(Uuid),
    Reset,
    Pause,
    Stop,
    Next,
    Previous,
}
