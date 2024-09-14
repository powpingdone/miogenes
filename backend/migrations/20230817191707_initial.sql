-- Initialization tables
-- NOTES:
-- all ids are UUIDs in byte format (16 bytes long)
CREATE TABLE IF NOT EXISTS user (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    username TEXT UNIQUE NOT NULL,
    -- phc string
    password TEXT NOT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS playlist (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    name TEXT NOT NULL,
    owner BLOB NOT NULL,
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS track (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    title TEXT NOT NULL,
    path TEXT NOT NULL,
    orig_fname TEXT NOT NULL,
    disk INTEGER NULL,
    track INTEGER NULL,
    -- fk
    owner BLOB NOT NULL,
    album BLOB NULL,
    artist BLOB NULL,
    cover_art BLOB NULL,
    FOREIGN KEY(album) REFERENCES album(id),
    FOREIGN KEY(artist) REFERENCES artist(id),
    FOREIGN KEY(cover_art) REFERENCES cover_art(id),
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS artist (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    artist_name TEXT UNIQUE NOT NULL,
    sort_name TEXT NULL,
    -- fk
    owner BLOB NOT NULL,
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS album (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    title TEXT NOT NULL,
    sort_title TEXT NULL,
    -- fk
    owner BLOB NOT NULL,
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS cover_art (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    img_blob BLOB NOT NULL,
    -- sha256 hash of img_blob
    img_hash BLOB UNIQUE NOT NULL CHECK (length(img_hash) == 32),
    -- fk
    owner BLOB NOT NULL,
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS JOIN_playlist_track (
    playlist BLOB NOT NULL,
    track BLOB NOT NULL,
    FOREIGN KEY(playlist) REFERENCES playlist(id),
    FOREIGN KEY(track) REFERENCES track(id)
) STRICT;
CREATE TABLE IF NOT EXISTS auth_keys (
    expiry INTEGER NOT NULL,
    secret BLOB NOT NULL,
    -- fk
    id BLOB NOT NULL CHECK (length(id) == 16),
    FOREIGN KEY(id) REFERENCES user(id)
) STRICT;
CREATE INDEX IF NOT EXISTS auth_keys_id ON auth_keys (id);
