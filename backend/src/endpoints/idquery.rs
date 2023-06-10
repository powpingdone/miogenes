use crate::{db::uuid_serialize, *};
use axum::extract::State;
use mio_common::*;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/albums", get(get_albums))
        .route("/playlists", get(get_playlists))
}

async fn get_albums(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, ..  }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            let mut conn = state.db.acquire().await?;
            retstructs::Albums {
                albums: sqlx::query!(
                    "SELECT DISTINCT album.id FROM album 
            JOIN track ON track.album = album.id 
            WHERE track.owner = ?;",
                    userid
                )
                .fetch_all(&mut *conn)
                .await?
                .into_iter()
                .map(|x| uuid_serialize(&x.id))
                .collect::<Result<_, _>>()?,
            }
        }),
    ))
}

async fn get_playlists(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, ..  }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    Ok::<_, MioInnerError>((
        StatusCode::OK,
        Json({
            let mut conn = state.db.acquire().await?;
            retstructs::Playlists {
                lists: sqlx::query!("SELECT DISTINCT id FROM playlist WHERE owner = ?;", userid)
                    .fetch_all(&mut *conn)
                    .await?
                    .into_iter()
                    .map(|x| uuid_serialize(&x.id))
                    .collect::<Result<_, _>>()?,
            }
        }),
    ))
}
