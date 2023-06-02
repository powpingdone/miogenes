use std::collections::HashSet;
use axum::extract::State;
use mio_common::*;
use crate::*;

pub fn routes() -> Router<MioState> {
    Router::new().route("/albums", get(get_albums)).route("/playlists", get(get_playlists))
}

async fn get_albums(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(
            retstructs::Albums,
            "SELECT album.id FROM album 
            JOIN track ON track.album = album.id 
            WHERE track.owner = ?;",
            userid
        )
            .fetch_one(conn)
            .await?
    })))
}

async fn get_playlists(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((StatusCode::OK, Json({
        let conn = state.db.acquire().await?;
        sqlx::query_as!(retstructs::Playlists, "SELECT id FROM playlist WHERE owner = ?;", userid)
            .fetch_one(conn)
            .await?
    })))
}
