use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use mio_common::*;
use mio_entity::*;
use sea_orm::*;
use crate::db::WebOut;
use crate::*;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/ti", get(track_info))
        .route("/ai", get(album_info))
        .route("/pi", get(playlist_info))
        .route("/ca", get(cover_art))
        .route("/ar", get(artist))
}

async fn track_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>(
        (
            StatusCode::OK,
            Json(
                Track::find_by_id(id)
                    .filter(track::Column::Owner.eq(key.id))
                    .one(&state.db)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        Into::<StatusCode>::into(
                            MioInnerError::NotFound(Level::Debug, anyhow!("could not find track {id}")),
                        )
                    })?
                    .web_out(&state.db)
                    .await,
            ),
        ),
    )
}

async fn album_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery { id: album }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>(
        (
            StatusCode::OK,
            Json(
                Album::find_by_id(album)
                    .join(JoinType::Join, album::Relation::Track.def())
                    .filter(track::Column::Owner.eq(key.id))
                    .one(&state.db)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        Into::<StatusCode>::into(
                            MioInnerError::NotFound(Level::Debug, anyhow!("could not find album {album}")),
                        )
                    })?
                    .web_out(&state.db)
                    .await,
            ),
        ),
    )
}

async fn playlist_info(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery { id: playlist }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>(
        (
            StatusCode::OK,
            Json(
                Playlist::find_by_id(playlist)
                    .filter(playlist::Column::Owner.eq(key.id))
                    .one(&state.db)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        Into::<StatusCode>::into(
                            MioInnerError::NotFound(Level::Debug, anyhow!("could not find playlist {playlist}")),
                        )
                    })?
                    .web_out(&state.db)
                    .await,
            ),
        ),
    )
}

async fn cover_art(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery { id: cover_art }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>(
        (
            StatusCode::OK,
            Json(
                CoverArt::find_by_id(cover_art)
                    .join(JoinType::Join, cover_art::Relation::Track.def())
                    .filter(track::Column::Owner.eq(key.id))
                    .one(&state.db)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        Into::<StatusCode>::into(
                            MioInnerError::NotFound(Level::Debug, anyhow!("could not find cover art {cover_art}")),
                        )
                    })?
                    .web_out(&state.db)
                    .await,
            ),
        ),
    )
}

async fn artist(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::IdInfoQuery { id: artist }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>(
        (
            StatusCode::OK,
            Json(
                Artist::find_by_id(artist)
                    .join(JoinType::Join, artist::Relation::Track.def())
                    .filter(track::Column::Owner.eq(key.id))
                    .one(&state.db)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        Into::<StatusCode>::into(
                            MioInnerError::NotFound(Level::Debug, anyhow!("could not find artist {artist}")),
                        )
                    })?
                    .web_out(&state.db)
                    .await,
            ),
        ),
    )
}
