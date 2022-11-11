use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserToken(#[serde(rename = "token")] pub Uuid);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrackInfoQuery(#[serde(rename = "tr")] pub Uuid);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AlbumInfoQuery(#[serde(rename = "au")] pub Uuid);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlaylistQuery(
    #[serde(rename = "id")] pub Uuid,
    // only send metadata, like track length and/or picture
    // default (false): send all tracks
    #[serde(rename = "md")]
    #[serde(default)]
    pub bool,
);
