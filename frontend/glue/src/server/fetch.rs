use crate::{error::ErrorSplit, MioClientState};
use mio_common::*;
use uuid::Uuid;

impl MioClientState {
    // TODO: playlists
    pub fn fetch_all_albums(&self) -> Result<retstructs::Albums, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/load/albums", self.url)))
            .call()?
            .into_json()?)
    }

    pub fn get_track_data(&self, id: Uuid) -> Result<retstructs::Track, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_album_data(&self, id: Uuid) -> Result<retstructs::Album, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/album?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_cover_art_data(&self, id: Uuid) -> Result<retstructs::CoverArt, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/coverart?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_artist_data(&self, id: Uuid) -> Result<retstructs::Artist, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/artist?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }
}
