use crate::db::uuid_serialize;
use crate::*;
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use mio_common::*;
use sqlx::Connection;

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
        let mut conn = state.db.acquire().await?;
        sqlx::query!("SELECT * FROM track WHERE id = ? AND owner = ?;", id, userid)
            .fetch_optional(&mut *conn)
            .await?
            .map(|x| retstructs::Track {
                id,
                album: x.album.map(|album| uuid_serialize(&album).unwrap()),
                cover_art: x.cover_art.map(|cover_art| uuid_serialize(&cover_art).unwrap()),
                artist: x.artist.map(|artist| uuid_serialize(&artist).unwrap()),
                title: x.title,
                disk: x.disk,
                track: x.track,
                tags: serde_json::from_str(&x.tags).unwrap(),
            })
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find track {id}")))?
    })))
}

async fn album_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        state.db.acquire().await?.transaction::<_, _, MioInnerError>(|txn| {
            Box::pin(async move {
                Ok(retstructs::Album {
                    id,
                    title: sqlx::query!(
                        "SELECT album.title FROM album 
                        JOIN track ON track.album = album.id 
                        WHERE album.id = ? AND track.owner = ?;",
                        id,
                        userid
                    )
                        .fetch_optional(&mut *txn)
                        .await?
                        .map(|x| x.title)
                        .ok_or_else(|| {
                            MioInnerError::NotFound(anyhow!("could not find album {id}"))
                        })?,
                    tracks: sqlx::query!(
                        "SELECT track.id FROM track
                        JOIN album ON track.album = album.id
                        WHERE album.id = ? AND track.owner=?;",
                        id,
                        userid
                    )
                        .fetch_all(&mut *txn)
                        .await?
                        .into_iter()
                        .map(|x| uuid_serialize(&x.id).unwrap())
                        .collect(),
                })
            })
        }).await?
    })))
}

async fn playlist_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>(
        (StatusCode::OK, Json(state.db.acquire().await?.transaction::<_, _, MioInnerError>(|txn| {
            Box::pin(async move {
                Ok(retstructs::Playlist {
                    id,
                    tracks: sqlx::query!(
                        "SELECT track FROM JOIN_playlist_track
                    JOIN playlist ON playlist.id = JOIN_playlist_track.playlist 
                    WHERE playlist = ? AND owner = ?;",
                        id,
                        userid
                    )
                        .fetch_all(&mut *txn)
                        .await?
                        .into_iter()
                        .map(|x| uuid_serialize(&x.track).unwrap())
                        .collect(),
                    name: sqlx::query!("SELECT name FROM playlist WHERE id = ? AND owner = ?;", id, userid)
                        .fetch_optional(&mut *txn)
                        .await?
                        .map(|x| x.name)
                        .ok_or_else(|| {
                            MioInnerError::NotFound(anyhow!("could not find playlist id {id}"))
                        })?,
                })
            })
        }).await?)),
    )
}

async fn cover_art(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let mut conn = state.db.acquire().await?;
        sqlx::query!(
            "SELECT webm_blob FROM cover_art 
            JOIN track ON track.cover_art = cover_art.id 
            WHERE cover_art.id = ? AND track.owner = ?;",
            id,
            userid
        )
            .fetch_optional(&mut *conn)
            .await?
            .map(|x| retstructs::CoverArt {
                id,
                webm_blob: x.webm_blob,
            })
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find cover art {id}")))?
    })))
}

async fn artist(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let mut conn = state.db.acquire().await?;
        sqlx::query!(
            "SELECT artist_name, artist.sort_name FROM artist 
                JOIN track ON track.artist = artist.id 
                WHERE artist.id = ? AND track.owner = ?;",
            id,
            userid
        )
            .fetch_optional(&mut *conn)
            .await?
            .map(|x| retstructs::Artist {
                id,
                name: x.artist_name,
                sort_name: x.sort_name,
            })
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find artist {id}")))?
    })))
}
