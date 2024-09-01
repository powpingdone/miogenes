use crate::db::uuid_serialize;
use crate::*;
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
#[allow(unused)]
use log::*;
use mio_protocol::*;
use sqlx::Connection;
use std::collections::HashSet;
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/track", get(track_info))
        .route("/album", get(album_info))
        .route("/playlist", get(playlist_info))
        .route("/coverart", get(cover_art))
        .route("/artist", get(artist_info))
        .route("/closest", get(closest_track))
}

fn uuid_map_back(x: Option<Vec<u8>>) -> Result<Option<Uuid>, MioInnerError> {
    match x {
        Some(x) => Ok(Some(uuid_serialize(&x)?)),
        None => Ok(None),
    }
}

#[tracing::instrument]
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

#[tracing::instrument]
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
                            .fetch_optional(txn.as_mut())
                            .await?
                            .map(|x| x.title)
                            .ok_or_else(|| {
                                MioInnerError::NotFound(anyhow!("could not find album {id}"))
                            })?,
                            tracks: sqlx::query!(
                                "SELECT track.id FROM track
                                JOIN album ON track.album = album.id
                                WHERE album.id = ? AND track.owner = ?;",
                                id,
                                userid
                            )
                            .fetch_all(txn.as_mut())
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

#[tracing::instrument]
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
                            .fetch_all(txn.as_mut())
                            .await?
                            .into_iter()
                            .map(|x| uuid_serialize(&x.track))
                            .collect::<Result<_, _>>()?,
                            name: sqlx::query!(
                                "SELECT name FROM playlist WHERE id = ? AND owner = ?;",
                                id,
                                userid
                            )
                            .fetch_optional(txn.as_mut())
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

#[tracing::instrument]
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

#[tracing::instrument]
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

#[tracing::instrument]
async fn closest_track(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Json(msgstructs::ClosestTrack {
        id,
        mut ignore_tracks,
    }): Json<msgstructs::ClosestTrack>,
) -> Result<impl IntoResponse, MioInnerError> {
    use futures::TryStreamExt;

    debug!("/query/closest finding closest for id {id}");
    state.db.acquire().await?.transaction(|txn| {
        Box::pin(async move {
            // fetch initial track_vec
            let cmp_track_vec =
                sqlx::query!(
                    "SELECT track_vec FROM track 
                    WHERE owner = ? AND id = ?;",
                    userid,
                    id
                )
                    .fetch_optional(txn.as_mut())
                    .await?
                    .ok_or_else(|| MioInnerError::NotFound(anyhow!("No track corresponds to id {id}")))?
                    .track_vec
                    .chunks_exact(4)
                    .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
                    .collect::<Vec<_>>();

            // setup loop vars
            ignore_tracks.push(id);
            let ignore_tracks = ignore_tracks.into_iter().collect::<HashSet<_>>();
            let mut stream =
                sqlx::query!(
                    "SELECT track_vec, id FROM track
                    WHERE owner = ?;",
                    userid
                ).fetch(txn.as_mut());
            let (mut curr_id, mut cosim) = (Uuid::nil(), -1.0f32);

            // finding code
            loop {
                let Some(query) = stream.try_next().await ? else {
                    break
                };
                let id = uuid_map_back(Some(query.id))?.unwrap();
                if ignore_tracks.contains(&id) {
                    continue;
                }
                let track_vec =
                    query
                        .track_vec
                        .chunks_exact(4)
                        .map(|x| f32::from_le_bytes(x.try_into().unwrap()))
                        .collect::<Vec<_>>();

                // compute cosine sim
                let cosim_comp =
                    (cmp_track_vec.iter().zip(&track_vec).map(|(a, b)| a * b).sum::<f32>()) /
                        (cmp_track_vec.iter().map(|x| x.powi(2)).sum::<f32>() *
                            track_vec.iter().map(|x| x.powi(2)).sum::<f32>()).sqrt();
                if cosim_comp > cosim {
                    trace!(
                        "/query/closest new cosim closest: {id} with {cosim_comp}, beating {curr_id} with {cosim}"
                    );
                    cosim = cosim_comp;
                    curr_id = id;
                }
            }
            if curr_id.is_nil() {
                return Err(MioInnerError::NotFound(anyhow!("no track was found that was similar.")));
            }
            debug!("/query/closest closest id found was {curr_id} with cosim {cosim}");
            Ok(Json(retstructs::ClosestId {
                id: curr_id,
                similarity: cosim,
            }))
        })
    }).await
}
