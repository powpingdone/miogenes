use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UserToken(String);