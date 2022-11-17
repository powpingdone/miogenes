use std::fmt::{Debug, Display};
use std::sync::Arc;

use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use serde::Serialize;
use uuid::Uuid;

use crate::db::*;
use crate::MioState;
use mio_common::*;

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
}

fn query_db<T>(
    debug_str: &str,
    state: Arc<MioState>,
    tree: Box<[u8]>,
    id: Uuid,
    table_id: Uuid,
) -> Result<(StatusCode, Json<Index<T>>), StatusCode>
where
    T: DbObject + Send + Clone + Debug + Serialize + IdTable,
    <T as DbObject>::Error: Display,
{
    tokio::task::block_in_place(|| {
        Ok::<_, StatusCode>((
            StatusCode::OK,
            Index::<T>::new_owned(
                id,
                table_id,
                &state
                    .db
                    .open_tree(tree.clone())
                    .map_err(|err| {
                        error!("{debug_str} failed to open tree {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .get(id.as_bytes())
                    .map_err(|err| {
                        error!("{debug_str} failed to select {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .ok_or_else(|| {
                        debug!("{debug_str} no track found for {id}");
                        StatusCode::NOT_FOUND
                    })?,
            )
            .map_err(|err| {
                error!("{debug_str} failed to serialize {err}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .web_out(),
        ))
    })
}

async fn track_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(msgstructs::TrackInfoQuery(id)): Query<msgstructs::TrackInfoQuery>,
) -> impl IntoResponse {
    query_db::<Track>(
        "GET /track/ti",
        state,
        UserTable::Track(key.id()).table(),
        id,
        key.id(),
    )
}

async fn album_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(msgstructs::AlbumInfoQuery(album)): Query<msgstructs::AlbumInfoQuery>,
) -> impl IntoResponse {
    query_db::<Album>(
        "GET /track/ai",
        state,
        UserTable::Album(key.id()).table(),
        album,
        key.id(),
    )
}

async fn playlist_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(msgstructs::PlaylistQuery(plquery)): Query<msgstructs::PlaylistQuery>,
) -> impl IntoResponse {
    query_db::<Playlist>(
        "GET /track/pi",
        state,
        UserTable::Playlist(key.id()).table(),
        plquery,
        key.id(),
    )
}

// return basic info of playlists
// ex: name, blurhash logo, id
async fn playlists(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
) -> impl IntoResponse {
    todo!()
}
