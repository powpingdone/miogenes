use std::sync::Arc;

use axum::extract::*;
use axum::response::IntoResponse;
use axum::routing::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::{MioState, User};

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
}

#[derive(Debug, Deserialize)]
struct TrackInfoQuery {
    #[serde(rename = "tr")]
    id: Uuid,
}

async fn track_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<User>,
    Query(track): Query<TrackInfoQuery>,
) -> impl IntoResponse {
    todo!()
}

#[derive(Debug, Deserialize)]
struct AlbumInfoQuery {
    #[serde(rename = "au")]
    id: Uuid,
}

async fn album_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<User>,
    Query(album): Query<AlbumInfoQuery>,
) -> impl IntoResponse {
    todo!()
}

// return basic info of playlists 
// ex: name, blurhash logo, id
async fn playlists(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<User>,
) -> impl IntoResponse {
    todo!()
}

#[derive(Debug, Deserialize)]
struct PlaylistQuery {
    id: Uuid,
    // only send metadata, like track length and/or picture
    // default (false): send all tracks 
    #[serde(rename = "md")]
    #[serde(default)]
    metadata: bool,
}

async fn playlist_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<User>,
    Query(plquery): Query<PlaylistQuery>,
) -> impl IntoResponse {
    todo!()
}
