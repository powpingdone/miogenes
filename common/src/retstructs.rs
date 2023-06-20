use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToResponse;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Track {
    pub id: Uuid,
    pub album: Option<Uuid>,
    pub cover_art: Option<Uuid>,
    pub artist: Option<Uuid>,
    pub title: String,
    pub disk: Option<i64>,
    pub track: Option<i64>,
    pub tags: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Album {
    pub id: Uuid,
    pub title: String,
    pub tracks: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Playlist {
    pub id: Uuid,
    pub tracks: Vec<Uuid>,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct CoverArt {
    pub id: Uuid,
    pub webm_blob: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Artist {
    pub id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Playlists {
    pub lists: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct UploadReturn {
    pub uuid: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct Albums {
    pub albums: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToResponse)]
pub struct FolderQuery {
    pub ret: Vec<String>, // this is beyond dumb, as this can be either paths or uuids
}