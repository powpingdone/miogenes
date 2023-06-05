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
    pub path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct FolderRename {
    pub path: String,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TrackMove {
    pub id: Uuid,
    pub new_path: String,
}
