use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use mio_common::*;
use sqlx::Connection;
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
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(retstructs::Track, "SELECT * FROM track WHERE id = ? AND owner = ?;", id, userid)
            .fetch_optional(conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find track {id}")))?
    })))
}

async fn album_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(
            retstructs::Album,
            "
            SELECT * FROM album 
            JOIN track ON track.album = album.id 
            WHERE album.id = ? AND track.owner = ?;
            ",
            id,
            userid
        )
            .fetch_optional(conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find album {id}")))?
    })))
}

async fn playlist_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json(state.db.acquire().await?.transaction(|txn| Box::pin(async move {
        retstructs::Playlist {
            id,
            tracks: sqlx::query!("SELECT track FROM JOIN_playlist_track WHERE playlist = ?;", id)
                .fetch_all(&mut *txn)
                .await?,
            name: sqlx::query!("SELECT name FROM playlist WHERE id = ? AND owner = ?;", id, userid)
                .fetch_optional(&mut *txn)
                .await?
                .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find playlist id {id}"))?),
        }
    })))))
}

async fn cover_art(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(
            retstructs::CoverArt,
            "
            SELECT * FROM cover_art 
            JOIN track ON track.cover_art = cover_art.id 
            WHERE cover_art.id = ? AND track.owner = ?;
            ",
            id,
            userid
        )
            .fetch_optional(conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find cover art {id}")))?
    })))
}

async fn artist(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(
            retstructs::Artist,
            "
            SELECT * FROM artist 
            JOIN track ON track.artist = artist.id 
            WHERE artist.id = ? AND track.owner = ?;
            ",
            id,
            userid
        )
            .fetch_optional(conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find artist {id}")))?
    })))
}
