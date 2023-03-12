use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserToken {
    pub token: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdInfoQuery {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteQuery {
    pub id: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackUploadQuery {
    pub fname: Option<String>,
}
