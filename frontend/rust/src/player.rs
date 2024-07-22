// frontend interaction with audio player
pub struct AudioPlayer {}

impl AudioPlayer {
    pub fn new() -> Result<Self, gstreamer::glib::Error> {
        gstreamer::init()?;
        Ok(Self {})
    }
}
