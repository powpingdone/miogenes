use std::sync::Arc;

use axum::extract::*;
use axum::response::IntoResponse;
use axum::routing::*;
use serde::Deserialize;
use uuid::Uuid;

use crate::db::{Index, User};
use crate::MioState;
use mio_common::*;

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
}

async fn track_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(track): Query<msgstructs::TrackInfoQuery>,
) -> impl IntoResponse {
    todo!()
}

async fn album_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(album): Query<msgstructs::AlbumInfoQuery>,
) -> impl IntoResponse {
    todo!()
}

// return basic info of playlists
// ex: name, blurhash logo, id
async fn playlists(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
) -> impl IntoResponse {
    todo!()
}

async fn playlist_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(plquery): Query<msgstructs::PlaylistQuery>,
) -> impl IntoResponse {
    todo!()
}
