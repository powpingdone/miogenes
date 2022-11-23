use std::fmt::{Debug, Display};
use std::sync::Arc;

use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use uuid::Uuid;

use crate::db::WebOut;
use crate::MioState;
use mio_common::*;
use mio_entity::*;

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
        .route("/aa", get(album_art))
        .route("/ar", get(artist))
}

async fn track_info(
    Extension(state): Extension<Arc<MioState>>,
    Extension(key): Extension<User>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Track::get_by_id()
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
                let mut ret = vec![];
                for poss in state
                    .db
                    .open_tree(UserTable::Playlist(key.id()).table())
                    .map_err(|err| {
                        error!("GET /track/pl could not open table {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?
                    .iter()
                {
                    let (key, _) = poss.map_err(|err| {
                        error!("GET /track/pl failed to serialize uuid {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                    ret.push(Uuid::from_slice(&key).map_err(|err| {
                        error!("GET /track/pl failed to serialize uuid {err}");
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?);
                }
                ret
            },
        }),
    ))
}
