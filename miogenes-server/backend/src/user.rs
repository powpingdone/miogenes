use axum_login::AuthUser;
use serde::*;
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use uuid::Uuid;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct User {
    #[serde(rename = "i")]
    pub userid: Uuid,
    #[serde(rename = "u")]
    pub username: String,
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(rename = "h")]
    pub password: [u8; 32],
}

impl AuthUser for User {
    fn get_id(&self) -> String {
        self.userid.to_string()
    }

    fn get_password_hash(&self) -> String {
        self.password
            .iter()
            .map(|x| format!("{x:01x}"))
            .fold("".to_owned(), |accum, x| accum + &x)
    }
}
