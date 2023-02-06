use crate::*;
use axum::extract::State;
use axum::routing::*;
use mio_entity::*;
use sea_orm::*;

pub fn routes() -> Router<MioState> {
    Router::new().route("/albums", get(get_albums))
}

async fn get_albums(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
) -> impl IntoResponse {
    Ok::<_, StatusCode>((StatusCode::OK, Album::find().stream(&state.db)))
}
