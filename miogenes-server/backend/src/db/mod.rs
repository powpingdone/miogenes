use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub enum UserTable {
    Track(Uuid),
    Album(Uuid),
    AlbumArt(Uuid),
    Artist(Uuid),
    Playlist(Uuid),
}

#[derive(Clone, Debug)]
pub enum TopLevel {
    User,
    UserToken,
    IndexUsernameToUser,
}

impl DbTable for UserTable {
    fn table(&self) -> Box<[u8]> {
        let (idx, x): (Uuid, &[u8]) = match self {
            UserTable::Track(id) => (*id, b"tracks"),
            UserTable::Album(id) => (*id, b"album"),
            UserTable::AlbumArt(id) => (*id, b"albumart"),
            UserTable::Artist(id) => (*id, b"artist"),
            UserTable::Playlist(id) => (*id, b"playlist"),
        };
        [x, b"-", idx.to_string().as_bytes()]
            .concat()
            .as_slice()
            .into()
    }
}

impl DbTable for TopLevel {
    fn table(&self) -> Box<[u8]> {
        match self {
            TopLevel::User => b"user".as_slice(),
            TopLevel::UserToken => b"usertoken".as_slice(),
            TopLevel::IndexUsernameToUser => b"idxuntou".as_slice(),
        }
        .into()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct User {
    username: String,
    password: String,
}

impl DbTable for User {
    fn table(&self) -> Box<[u8]> {
        TopLevel::User.table()
    }
}

impl User {
    pub fn generate(username: String, password: String) -> Vec<u8> {
        serde_json::to_vec(&Self { username, password }).unwrap()
    }
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserToken {
    expiry: DateTime<Utc>,
    user: Uuid,
}

impl DbTable for UserToken {
    fn table(&self) -> Box<[u8]> {
        TopLevel::UserToken.table()
    }
}

impl UserToken {
    pub fn generate(user: Uuid, expiry: DateTime<Utc>) -> Vec<u8> {
        serde_json::to_vec(&Self { user, expiry }).unwrap()
    }

    pub fn push_forward(&mut self, t: Duration) {
        self.expiry = Utc::now() + t;
    }

    pub fn is_expired(&self) -> bool {
        self.expiry < Utc::now()
    }

    pub fn user(&self) -> Uuid {
        self.user
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Track {
    album: Option<Uuid>,
    cover_art: Option<Uuid>,
    artist: Option<Uuid>,
    title: String,
    tags: HashMap<String, String>,
}

impl IdTable for Track {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Track(id).table()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Album {
    artist: Vec<Uuid>,
    title: String,
    track: Vec<Uuid>,
}

impl IdTable for Album {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Album(id).table()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Playlist {
    tracks: Vec<Uuid>,
    name: String,
}

impl IdTable for Playlist {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Playlist(id).table()
    }
}

mod types;
pub(crate) use types::*;

mod migrations;
pub(crate) use migrations::migrate;
