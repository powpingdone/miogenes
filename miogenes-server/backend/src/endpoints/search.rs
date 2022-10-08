use std::sync::Arc;

use axum::extract::ws::WebSocket;
use axum::extract::{Query, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::Response;
use axum::{response::IntoResponse, *};
use log::*;
use uuid::Uuid;

pub async fn search(
    Extension(state): Extension<Arc<crate::MioState>>,
    ws: WebSocketUpgrade,
    Query(key): Query<crate::User>,
) -> Result<Response, impl IntoResponse> {
    let userid = crate::login_check(state.db.clone(), key).await?;
    Ok::<_, (StatusCode, Json<crate::MioError>)>(
        ws.on_upgrade(move |x| search_inner(x, state, userid)),
    )
}

async fn search_inner(mut ws: WebSocket, state: Arc<crate::MioState>, userid: Uuid) {
    // idea:
    // dynamic keystroke sending and cancelling for db searching
    // each keystroke/string update is sent to here over the websocket.
    // drop the future being polled on such an update and spin a new future
    //
    // research: dropping future to poll, is db SELECT cancellation safe?
    // is this also reactive to when new tracks get uploaded?
    while let Some(msg) = ws.recv().await {
        if let Err(err) = msg {
            info!("search inner connection closed: {err}");
            return;
        }
        let msg = msg.unwrap();
    }
}