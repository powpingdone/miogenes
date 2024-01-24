use crate::{error::GlueResult, MioClientState};
use mio_common::*;
use uuid::Uuid;

impl MioClientState {
    // TODO: playlists
    pub async fn fetch_all_albums(&self) -> GlueResult<retstructs::Albums> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/load/albums", self.url)))
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_track_data(&self, id: Uuid) -> GlueResult<retstructs::Track> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/track", self.url,)))
            .query(&msgstructs::IdInfoQuery { id })
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_album_data(&self, id: Uuid) -> GlueResult<retstructs::Album> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/album", self.url,)))
            .query(&msgstructs::IdInfoQuery { id })
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_cover_art_data(&self, id: Uuid) -> GlueResult<retstructs::CoverArt> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/coverart", self.url,)))
            .query(&msgstructs::IdInfoQuery { id })
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_artist_data(&self, id: Uuid) -> GlueResult<retstructs::Artist> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/artist", self.url,)))
            .query(&msgstructs::IdInfoQuery { id })
            .send()
            .await?
            .json()
            .await?)
    }

    pub async fn get_closest(
        &self,
        id: Uuid,
        ignore_tracks: Vec<Uuid>,
    ) -> GlueResult<retstructs::ClosestId> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/query/closest", self.url)))
            .json(&msgstructs::ClosestTrack { id, ignore_tracks })
            .send()
            .await?
            .json()
            .await?)
    }

    pub fn stream(&self, id: Uuid) -> GlueResult<reqwest::blocking::Response> {
        Ok(reqwest::blocking::Client::new().get(
            &format!("{}/api/track", self.url))
                .bearer_auth(self.key.get().unwrap().to_string())
                .query(&msgstructs::IdInfoQuery { id })
                .send()?
        )
    }
}
