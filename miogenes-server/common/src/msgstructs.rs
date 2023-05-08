use serde::{
    Deserialize,
    Serialize,
};
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
    pub fname: Option<String>,
}
