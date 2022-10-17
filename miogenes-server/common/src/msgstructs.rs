use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct UserToken(#[serde(rename = "token")] pub Uuid);
