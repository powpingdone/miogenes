use std::fmt::{Debug, Display};
use std::sync::Arc;

use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use serde::Serialize;
use uuid::Uuid;

use crate::db::{WebOut, *};
use crate::MioState;
use mio_common::*;

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
        .route("/aa", get(album_art))
        .route("/ar", get(artist))
}

fn query_db<T>(
    debug_str: &str,
    state: Arc<MioState>,
    tree: Box<[u8]>,
    id: Uuid,
    table_id: Uuid,
) -> Result<(StatusCode, Json<retstructs::Index<<T as WebOut>::WebOut>>), StatusCode>
where
    T: WebOut + DbObject + Send + Clone + Debug + Serialize + IdTable,
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
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
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
    Query(msgstructs::IdInfoQuery(album)): Query<msgstructs::IdInfoQuery>,
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
    Query(msgstructs::IdInfoQuery(plquery)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    query_db::<Playlist>(
        "GET /track/pi",
        state,
        UserTable::Playlist(key.id()).table(),
        plquery,
        key.id(),
    )
}

async fn album_art(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    query_db::<AlbumArt>(
        "GET /track/pi",
        state,
        UserTable::Playlist(key.id()).table(),
        id,
        key.id(),
    )
}

async fn artist(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    query_db::<Artist>(
        "GET /track/ar",
        state,
        UserTable::Artist(key.id()).table(),
        id,
        key.id(),
    )
}

// return all playlists
async fn playlists(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<Index<User>>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(retstructs::Playlists {
            lists: {
                let mut x = vec![];
                for poss in state
                    .db
                    .open_tree(UserTable::Playlist(key.id()).table())
                    .map_err(|err| {
                        error!("/track/pl could not open table {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .iter()
                {
                    let (key, _) = poss.map_err(|err| {
                        error!("/track/pl failed to serialize uuid {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                    x.push(Uuid::from_slice(&key).map_err(|err| {
                        error!("/track/pl failed to serialize uuid {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?);
                }
                x
            },
        }),
    ))
}
