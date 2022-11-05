pub enum Table {
    Tracks,
    Album,
    AlbumArt,
    Artist,
    User,
    UserToken,
}

impl AsRef<[u8]> for Table {
    fn as_ref(&self) -> &[u8] {
        match self {
            Table::Tracks => b"tracks",
            Table::Album => b"album",
            Table::AlbumArt => b"albumart",
            Table::Artist => b"artist",
            Table::User => b"user",
            Table::UserToken => b"usertoken",
        }
    }
}

mod types;
pub(crate) use types::*;

mod migrations;
pub(crate) use migrations::migrate;
