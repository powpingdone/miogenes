
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use sea_orm::*;

use crate::db::WebOut;
use crate::{db_err, MioInnerError, MioState};
use mio_common::*;
use mio_entity::*;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pl", get(playlists))
        .route("/pi", get(playlist_info))
        .route("/ca", get(cover_art))
        .route("/ar", get(artist))
}

async fn track_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(
            Track::find_by_id(id)
                .filter(track::Column::Owner.eq(key.id))
                .one(&state.db)
                .await
                .map_err(db_err)?
                .ok_or_else(|| {
                    Into::<StatusCode>::into(MioInnerError::NotFound(
                        Level::Debug,
                        anyhow!("could not find track {id}"),
                    ))
                })?
                .web_out(&state.db)
                .await,
        ),
    ))
}

async fn album_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery(album)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(
            Album::find_by_id(album)
                .join(JoinType::Join, track::Relation::Album.def())
                .filter(track::Column::Owner.eq(key.id))
                .one(&state.db)
                .await
                .map_err(db_err)?
                .ok_or_else(|| {
                    Into::<StatusCode>::into(MioInnerError::NotFound(
                        Level::Debug,
                        anyhow!("could not find album {album}"),
                    ))
                })?
                .web_out(&state.db)
                .await,
        ),
    ))
}

async fn playlist_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(
            Playlist::find_by_id(id)
                .filter(playlist::Column::Owner.eq(key.id))
                .one(&state.db)
                .await
                .map_err(db_err)?
                .ok_or_else(|| {
                    Into::<StatusCode>::into(MioInnerError::NotFound(
                        Level::Debug,
                        anyhow!("could not find album {id}"),
                    ))
                })?
                .web_out(&state.db)
                .await,
        ),
    ))
}

async fn cover_art(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(
            CoverArt::find_by_id(id)
                .join(JoinType::Join, track::Relation::CoverArt.def())
                .filter(track::Column::Owner.eq(key.id))
                .one(&state.db)
                .await
                .map_err(db_err)?
                .ok_or_else(|| {
                    Into::<StatusCode>::into(MioInnerError::NotFound(
                        Level::Debug,
                        anyhow!("could not find album {id}"),
                    ))
                })?
                .web_out(&state.db)
                .await,
        ),
    ))
}

async fn artist(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery(id)): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(
            Artist::find_by_id(id)
                .join(JoinType::Join, track::Relation::Artist.def())
                .filter(track::Column::Owner.eq(key.id))
                .one(&state.db)
                .await
                .map_err(db_err)?
                .ok_or_else(|| {
                    Into::<StatusCode>::into(MioInnerError::NotFound(
                        Level::Debug,
                        anyhow!("could not find album {id}"),
                    ))
                })?
                .web_out(&state.db)
                .await,
        ),
    ))
}

// return all playlists
async fn playlists(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(retstructs::Playlists {
            lists: {
                Playlist::find()
                    .filter(playlist::Column::Owner.eq(key.id))
                    .all(&state.db)
                    .await
                    .map_err(db_err)?
                    .into_iter()
                    .map(|x| x.id)
                    .collect()
            },
        }),
    ))
}
