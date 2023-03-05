use mio_common::retstructs;
use mio_entity::*;
use sea_orm::*;

// send out the type
#[async_trait::async_trait]
pub trait WebOut {
    type WebOut;

    async fn web_out(self, db: &DatabaseConnection) -> Self::WebOut;
}

#[async_trait::async_trait]
impl WebOut for mio_entity::track::Model {
    type WebOut = retstructs::Track;

    async fn web_out(self, _: &DatabaseConnection) -> Self::WebOut {
        retstructs::Track {
            id: self.id,
            title: self.title,
            album: self.album,
            cover_art: self.cover_art,
            artist: self.artist,
            sort_name: self.sort_name,
            tags: self
                .tags
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.to_owned(), v.to_string()))
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl WebOut for mio_entity::album::Model {
    type WebOut = retstructs::Album;

    async fn web_out(self, db: &DatabaseConnection) -> Self::WebOut {
        retstructs::Album {
            title: self.title,
            id: self.id,
            tracks: mio_entity::Track::find()
                .filter(mio_entity::track::Column::Album.eq(self.id))
                .all(db)
                .await
                .expect("DATABASE Error: {}")
                .into_iter()
                .map(|x| x.id)
                .collect(),
        }
    }
}

#[async_trait::async_trait]
impl WebOut for mio_entity::cover_art::Model {
    type WebOut = retstructs::CoverArt;

    async fn web_out(self, _: &DatabaseConnection) -> Self::WebOut {
        retstructs::CoverArt {
            id: self.id,
            data: self.webm_blob,
        }
    }
}

#[async_trait::async_trait]
impl WebOut for mio_entity::artist::Model {
    type WebOut = retstructs::Artist;

    async fn web_out(self, _: &DatabaseConnection) -> Self::WebOut {
        retstructs::Artist {
            id: self.id,
            name: self.name,
            sort_name: self.sort_name,
        }
    }
}

#[async_trait::async_trait]
impl WebOut for playlist::Model {
    type WebOut = retstructs::Playlist;

    async fn web_out(self, db: &DatabaseConnection) -> Self::WebOut {
        retstructs::Playlist {
            id: self.id,
            name: self.name,
            tracks: JoinPlaylistTrack::find()
                .filter(join_playlist_track::Column::PlaylistId.eq(self.id))
                .all(db)
                .await
                .expect("DATABASE ERROR: {}")
                .into_iter()
                .map(|x| x.track_id)
                .collect(),
        }
    }
}
