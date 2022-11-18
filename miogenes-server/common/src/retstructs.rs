use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Track {
    pub album: Option<Uuid>,
    pub cover_art: Option<Uuid>,
    pub artist: Option<Uuid>,
    pub title: String,
    pub tags: HashMap<String, String>,
    pub sort_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Album {
    pub artist: Vec<Uuid>,
    pub title: String,
    pub tracks: Vec<Uuid>,
    pub sort_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Playlist {
    pub tracks: Vec<Uuid>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AlbumArt {
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Artist {
    pub name: String,
    pub sort_name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Index<T> {
    pub id: Uuid,
    pub inner: T,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Playlists {
    pub lists: Vec<Uuid>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UploadReturn {
    pub uuid: Vec<Uuid>,
}
