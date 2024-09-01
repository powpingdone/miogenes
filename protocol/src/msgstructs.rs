use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct UserToken {
    pub token: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct IdInfoQuery {
    pub id: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DeleteQuery {
    pub id: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TrackUploadQuery {
    pub dir: String,
    pub fname: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FolderCreateDelete {
    pub name: String,
    pub path: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FolderQuery {
    pub path: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FolderRename {
    pub old_path: Vec<String>,
    pub new_path: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TrackMove {
    pub id: Uuid,
    pub new_path: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ClosestTrack {
    pub id: Uuid,
    pub ignore_tracks: Vec<Uuid>,
}
