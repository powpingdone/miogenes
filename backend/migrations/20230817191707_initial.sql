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
    path TEXT NOT NULL,
    owner BLOB NOT NULL,
    orig_fname TEXT NOT NULL,
    disk INTEGER NULL,
    track INTEGER NULL,
    -- extra tags, as json
    tags TEXT NOT NULL,
    album BLOB NULL,
    artist BLOB NULL,
    cover_art BLOB NULL,
    -- vector for position of the track
    track_vec BLOB NOT NULL CHECK (
        length(id) == (
            /* float */
            4 *
            /* vec length */
            100
        )
    ),
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
CREATE TABLE IF NOT EXISTS auth_keys (
    id BLOB NOT NULL CHECK (length(id) == 16),
    expiry INTEGER NOT NULL,
    secret BLOB NOT NULL,
    FOREIGN KEY(id) REFERENCES user(id)
) STRICT;
CREATE INDEX IF NOT EXISTS auth_keys_id ON auth_keys (id);