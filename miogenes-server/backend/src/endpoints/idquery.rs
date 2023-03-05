use crate::*;
use axum::extract::State;
use mio_common::*;
use mio_entity::*;
use sea_orm::*;

// TODO: turn these endpoints into a stream
//
// NOTE: some considerations for this
//
// 1. &state.db is not 'static
//
// 2. the stream can still return DbErr
//
// 3. does axum read out the stream as fast as possible?
pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/albums", get(get_albums))
        .route("/playlists", get(get_playlists))
}

async fn get_albums(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((
        StatusCode::OK,
        Json(retstructs::Albums {
            albums: Album::find()
                .filter(album::Column::Id.eq(key.id))
                .all(&state.db)
                .await
                .map_err(db_err)?
                .into_iter()
                .map(|x| x.id)
                .collect(),
        }),
    ))
}

async fn get_playlists(
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
