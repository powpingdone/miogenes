use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use mio_common::retstructs;
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
    sort_name: String,
}

impl IdTable for Track {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Track(id).table()
    }
}

impl WebOut for Track {
    type WebOut = retstructs::Track;
    fn web_out(self) -> Self::WebOut {
        retstructs::Track {
            title: self.title,
            album: self.album,
            cover_art: self.cover_art,
            artist: self.artist,
            tags: self.tags,
            sort_name: self.sort_name,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Album {
    artist: Vec<Uuid>,
    title: String,
    tracks: Vec<Uuid>,
    sort_name: String,
}

impl IdTable for Album {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Album(id).table()
    }
}

impl WebOut for Album {
    type WebOut = retstructs::Album;
    fn web_out(self) -> Self::WebOut {
        retstructs::Album {
            artist: self.artist,
            title: self.title,
            tracks: self.tracks,
            sort_name: self.sort_name,
        }
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

impl WebOut for Playlist {
    type WebOut = retstructs::Playlist;
    fn web_out(self) -> Self::WebOut {
        retstructs::Playlist {
            tracks: self.tracks,
            name: self.name,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AlbumArt {
    data: Vec<u8>,
}

impl IdTable for AlbumArt {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::AlbumArt(id).table()
    }
}

impl WebOut for AlbumArt {
    type WebOut = retstructs::AlbumArt;
    fn web_out(self) -> Self::WebOut {
        retstructs::AlbumArt { data: self.data }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Artist {
    name: String,
    sort_name: String,
}

impl IdTable for Artist {
    fn id_table(&self, id: Uuid) -> Box<[u8]> {
        UserTable::Artist(id).table()
    }
}

impl WebOut for Artist {
    type WebOut = retstructs::Artist;

    fn web_out(self) -> Self::WebOut {
        retstructs::Artist {
            name: self.name,
            sort_name: self.sort_name,
        }
    }
}

mod types;
pub(crate) use types::*;

mod migrations;
pub(crate) use migrations::migrate;
