use crate::{error::GlueResult, MioClientState};
use mio_common::*;
use std::io::Read;
use uuid::Uuid;

impl MioClientState {
    // TODO: playlists
    pub fn fetch_all_albums(&self) -> GlueResult<retstructs::Albums> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/load/albums", self.url)))
            .call()?
            .into_json()?)
    }

    pub fn get_track_data(&self, id: Uuid) -> GlueResult<retstructs::Track> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_album_data(&self, id: Uuid) -> GlueResult<retstructs::Album> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/album?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_cover_art_data(&self, id: Uuid) -> GlueResult<retstructs::CoverArt> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/coverart?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_artist_data(&self, id: Uuid) -> GlueResult<retstructs::Artist> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/artist?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn stream(&self, id: Uuid) -> GlueResult<Box<dyn Read + Send + Sync + 'static>> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_reader())
    }

    pub fn get_closest(
        &self,
        id: Uuid,
        ignore_tracks: Vec<Uuid>,
    ) -> GlueResult<retstructs::ClosestId> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/closest", self.url,)))
            .send_json(msgstructs::ClosestTrack { id, ignore_tracks })?
            .into_json()?)
    }
}
