use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub enum UserTable {
    Tracks,
    Album,
    AlbumArt,
    Artist,
}

#[derive(Clone)]
pub enum TopLevel {
    User,
    UserToken,
}

impl AsRef<[u8]> for UserTable {
    fn as_ref(&self) -> &[u8] {
        match self {
            UserTable::Tracks => b"tracks",
            UserTable::Album => b"album",
            UserTable::AlbumArt => b"albumart",
            UserTable::Artist => b"artist",
        }
    }
}

impl AsRef<[u8]> for TopLevel {
    fn as_ref(&self) -> &[u8] {
        match self {
            TopLevel::User => b"user",
            TopLevel::UserToken => b"usertoken",
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    username: String,
    password: String,
}

impl DbTable for User {
    const TABLE: TopLevel = TopLevel::User;
    type Ret = TopLevel;
}

impl User {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserToken {
    expiry: DateTime<Utc>,
    user: Uuid,
}

impl DbTable for UserToken {
    type Ret = TopLevel;
    const TABLE: TopLevel = TopLevel::UserToken;
}

impl UserToken {
    pub fn generate(user: Uuid, expiry: DateTime<Utc>) -> Vec<u8> {
        serde_json::to_vec(&Self { user, expiry }).unwrap()
    }

    pub fn push_forward(&mut self, t: Duration) {
        self.expiry += t;
    }

    pub fn is_expired(&self) -> bool {
        self.expiry < Utc::now()
    }

    pub fn user(&self) -> Uuid {
        self.user
    }
}

mod types;
pub(crate) use types::*;

mod migrations;
pub(crate) use migrations::migrate;
