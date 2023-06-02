-- Initialization tables
-- NOTES:
-- all ids are UUIDs in byte format (16 bytes long)
CREATE TABLE IF NOT EXISTS album (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    title TEXT NOT NULL,
    sort_title TEXT NULL
) STRICT;
CREATE TABLE IF NOT EXISTS cover_art (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    webm_blob BLOB NOT NULL,
    -- sha256 hash of previous info
    img_hash BLOB UNIQUE NOT NULL CHECK (length(img_hash) == 32)
) STRICT;
CREATE TABLE IF NOT EXISTS artist (
    id BLOB PRIMARY KEY NOT NULL CHECK (length(id) == 16),
    artist_name TEXT UNIQUE NOT NULL,
    sort_name TEXT NULL
) STRICT;
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
    sort_name TEXT NULL,
    disk INTEGER NULL,
    track INTEGER NULL,
    -- extra tags, as json
    tags TEXT NOT NULL,
    -- sha256 hash of the wavelength
    audio_hash BLOB UNIQUE NOT NULL CHECK (length(audio_hash) == 32),
    orig_fname TEXT NOT NULL,
    album BLOB NULL,
    artist BLOB NULL,
    cover_art BLOB NULL,
    owner BLOB NOT NULL,
    FOREIGN KEY(album) REFERENCES album(id),
    FOREIGN KEY(artist) REFERENCES artist(id),
    FOREIGN KEY(cover_art) REFERENCES cover_art(id),
    FOREIGN KEY(owner) REFERENCES user(id)
) STRICT;
CREATE TABLE IF NOT EXISTS JOIN_playlist_track (
    playlist BLOB NOT NULL,
    track BLOB NOT NULL,
    FOREIGN KEY(playlist) REFERENCES playlist(id),
    FOREIGN KEY(track) REFERENCES track(id)
) STRICT;