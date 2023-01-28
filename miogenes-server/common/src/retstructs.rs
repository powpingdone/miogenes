use serde::{
    Deserialize,
    Serialize,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Track {
    pub id: Uuid,
    pub album: Option<Uuid>,
    pub cover_art: Option<Uuid>,
    pub artist: Option<Uuid>,
    pub title: String,
    pub tags: HashMap<String, String>,
    pub sort_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Album {
    pub id: Uuid,
    pub title: String,
    pub tracks: Vec<Uuid>,
    pub sort_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Playlist {
    pub id: Uuid,
    pub tracks: Vec<Uuid>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CoverArt {
    pub id: Uuid,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Artist {
    pub id: Uuid,
    pub name: String,
    pub sort_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Playlists {
    pub lists: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UploadReturn {
    pub uuid: Vec<Uuid>,
}
