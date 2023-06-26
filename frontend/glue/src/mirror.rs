use flutter_rust_bridge::frb;
pub use mio_common::retstructs::{
    Album, Albums, Artist, CoverArt, ErrorMsg, FolderQuery, Playlist, Playlists, Track,
    UploadReturn,
};
use uuid::Uuid;

// this is a copy of mio_common::retstructs. frb makes sure that this matches

#[frb(mirror(Track))]
pub struct _Track {
    pub id: Uuid,
    pub album: Option<Uuid>,
    pub cover_art: Option<Uuid>,
    pub artist: Option<Uuid>,
    pub title: String,
    pub disk: Option<i64>,
    pub track: Option<i64>,
    //pub tags: RustOpaque<HashMap<String, String>>,
}

#[frb(mirror(Album))]
pub struct _Album {
    pub id: Uuid,
    pub title: String,
    pub tracks: Vec<Uuid>,
}

#[frb(mirror(Playlist))]
pub struct _Playlist {
    pub id: Uuid,
    pub tracks: Vec<Uuid>,
    pub name: String,
}

#[frb(mirror(CoverArt))]
pub struct _CoverArt {
    pub id: Uuid,
    pub webm_blob: Vec<u8>,
}

#[frb(mirror(Artist))]
pub struct _Artist {
    pub id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
}

#[frb(mirror(Playlists))]
pub struct _Playlists {
    pub lists: Vec<Uuid>,
}

#[frb(mirror(UploadReturn))]
pub struct _UploadReturn {
    pub uuid: Uuid,
}

#[frb(mirror(Albums))]
pub struct _Albums {
    pub albums: Vec<Uuid>,
}

#[frb(mirror(FolderQuery))]
pub struct _FolderQuery {
    // this is beyond dumb, as this can be either paths or uuids
    pub ret: Vec<String>,
}

#[frb(mirror(ErrorMsg))]
pub struct _ErrorMsg {
    pub error: String,
}
