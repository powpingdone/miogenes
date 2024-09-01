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
    pub uuid: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Albums {
    pub albums: Vec<Uuid>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FolderQuery {
    pub ret: FolderQueryItem,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FolderQueryItem {
    // if Some, then this is the contents of the folder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<Vec<FolderQueryItem>>,
    // name of the item
    pub id: String,
    // item is of type
    pub item_type: FolderQueryItemType,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FolderQueryItemType {
    // id is name of folder
    Folder = 0,
    // id is uuid of audio
    Audio = 1,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ErrorMsg {
    pub error: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ClosestId {
    pub id: Uuid,
    pub similarity: f32,
}
