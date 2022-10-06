use std::sync::Arc;

use axum::extract::*;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use serde::Deserialize;
use uuid::Uuid;

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
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(key): Query<crate::User>,
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
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(key): Query<crate::User>,
    Query(album): Query<AlbumInfoQuery>,
) -> impl IntoResponse {
    todo!()
}

// return basic info of playlists 
// ex: name, blurhash logo, id
async fn playlists(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(key): Query<crate::User>,
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
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(key): Query<crate::User>,
    Query(plquery): Query<PlaylistQuery>,
) -> impl IntoResponse {
    todo!()
}
