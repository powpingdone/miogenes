use crate::db::uuid_serialize;
use crate::*;
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
#[allow(unused)]
use log::*;
use mio_common::*;
use sqlx::Connection;
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/track", get(track_info))
        .route("/album", get(album_info))
        .route("/playlist", get(playlist_info))
        .route("/coverart", get(cover_art))
        .route("/artist", get(artist_info))
}

fn uuid_map_back(x: Option<Vec<u8>>) -> Result<Option<Uuid>, MioInnerError> {
    match x {
        Some(x) => Ok(Some(uuid_serialize(&x)?)),
        None => Ok(None),
    }
}

#[utoipa
    ::path(
        get,
        path = "/api/query/track",
        params(("id" = Uuid, Query, description = "Track id to query")),
        security(("jwt" = [])),
        responses(
            (status = 200, response = retstructs::Track),
            (status = 404, response = error::ErrorMsg),
            (status = 500, response = error::ErrorMsg),
        ),
    )]
async fn track_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            let mut conn = state.db.acquire().await?;
            let x = sqlx::query!(
                "SELECT * FROM track WHERE id = ? AND owner = ?;",
                id,
                userid
            )
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find track {id}")))?;
            retstructs::Track {
                id,
                album: uuid_map_back(x.album)?,
                cover_art: uuid_map_back(x.cover_art)?,
                artist: uuid_map_back(x.artist)?,
                title: x.title,
                disk: x.disk,
                track: x.track,
                tags: serde_json::from_str(&x.tags).map_err(|err| {
                    MioInnerError::DbError(anyhow!("could not serialize tags {err}"))
                })?,
            }
        }),
    ))
}

#[utoipa
    ::path(
        get,
        path = "/api/query/album",
        params(("id" = Uuid, Query, description = "Album id to query")),
        security(("jwt" = [])),
        responses(
            (status = 200, response = retstructs::Album),
            (status = 404, response = error::ErrorMsg),
            (status = 500, response = error::ErrorMsg),
        ),
    )]
async fn album_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            state
                .db
                .acquire()
                .await?
                .transaction::<_, _, MioInnerError>(|txn| {
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
                            .map(|x| uuid_serialize(&x.id))
                            .collect::<Result<_, _>>()?,
                        })
                    })
                })
                .await?
        }),
    ))
}

#[utoipa
    ::path(
        get,
        path = "/api/query/playlist",
        params(("id" = Uuid, Query, description = "Playlist id to query")),
        security(("jwt" = [])),
        responses(
            (status = 200, response = retstructs::Playlist),
            (status = 404, response = error::ErrorMsg),
            (status = 500, response = error::ErrorMsg),
        ),
    )]
async fn playlist_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json(
            state
                .db
                .acquire()
                .await?
                .transaction::<_, _, MioInnerError>(|txn| {
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
                            .map(|x| uuid_serialize(&x.track))
                            .collect::<Result<_, _>>()?,
                            name: sqlx::query!(
                                "SELECT name FROM playlist WHERE id = ? AND owner = ?;",
                                id,
                                userid
                            )
                            .fetch_optional(&mut *txn)
                            .await?
                            .map(|x| x.name)
                            .ok_or_else(|| {
                                MioInnerError::NotFound(anyhow!("could not find playlist id {id}"))
                            })?,
                        })
                    })
                })
                .await?,
        ),
    ))
}

#[utoipa
    ::path(
        get,
        path = "/api/query/coverart",
        params(("id" = Uuid, Query, description = "Cover Art to query")),
        security(("jwt" = [])),
        responses(
            (status = 200, response = retstructs::CoverArt),
            (status = 404, response = error::ErrorMsg),
            (status = 500, response = error::ErrorMsg),
        ),
    )]
async fn cover_art(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            let mut conn = state.db.acquire().await?;
            let x = sqlx::query!(
                "SELECT webm_blob FROM cover_art 
                JOIN track ON track.cover_art = cover_art.id 
                WHERE cover_art.id = ? AND track.owner = ?;",
                id,
                userid
            )
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find cover art {id}")))?;
            retstructs::CoverArt {
                id,
                webm_blob: x.webm_blob,
            }
        }),
    ))
}

#[utoipa
    ::path(
        get,
        path = "/api/query/artist",
        params(("id" = Uuid, Query, description = "Artist to query")),
        security(("jwt" = [])),
        responses(
            (status = 200, response = retstructs::Artist),
            (status = 404, response = error::ErrorMsg),
            (status = 500, response = error::ErrorMsg),
        ),
    )]
async fn artist_info(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            let mut conn = state.db.acquire().await?;
            let x = sqlx::query!(
                "SELECT artist_name, artist.sort_name FROM artist 
                JOIN track ON track.artist = artist.id 
                WHERE artist.id = ? AND track.owner = ?;",
                id,
                userid
            )
            .fetch_optional(&mut *conn)
            .await?
            .ok_or_else(|| MioInnerError::NotFound(anyhow!("could not find artist {id}")))?;
            retstructs::Artist {
                id,
                name: x.artist_name,
                sort_name: x.sort_name,
            }
        }),
    ))
}
