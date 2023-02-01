use serde::{
    Deserialize,
    Serialize,
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserToken(
    #[serde(rename = "token")]
    pub Uuid,
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdInfoQuery(
    #[serde(rename = "id")]
    pub Uuid,
);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteQuery {
    pub id: Uuid,
}
