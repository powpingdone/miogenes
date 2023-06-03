use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Album {
    pub id: Uuid,
    pub title: String,
    pub tracks: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    pub id: Uuid,
    pub tracks: Vec<Uuid>,
    pub name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CoverArt {
    pub id: Uuid,
    pub webm_blob: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Artist {
    pub id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Playlists {
    pub lists: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct UploadReturn {
    pub uuid: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Albums {
    pub albums: Vec<Uuid>,
}
