use axum::extract::ws::WebSocket;
use axum::extract::{
    State,
    WebSocketUpgrade,
};
use axum::http::StatusCode;
use axum::response::Response;
use axum::{
    response::IntoResponse,
    *,
};
use log::*;
use uuid::Uuid;
use crate::MioState;

pub async fn search(
    State(state): State<MioState>,
    ws: WebSocketUpgrade,
    Extension(userid): Extension<Uuid>,
) -> Result<Response, impl IntoResponse> {
    Ok::<_, StatusCode>(ws.on_upgrade(move |x| search_inner(x, state, userid)))
}

async fn search_inner(mut ws: WebSocket, state: MioState, userid: Uuid) {
    // idea: dynamic keystroke sending and cancelling for db searching each keystroke/string
    // update is sent to here over the websocket. drop the future being polled on such an
    // update and spin a new future
    //
    // research: dropping future to poll, is db SELECT cancellation safe? is this also
    // reactive to when new tracks get uploaded?
    while let Some(msg) = ws.recv().await {
        if let Err(err) = msg {
            info!("search inner connection closed: {err}");
            return;
        }
        let msg = msg.unwrap();
    }
}
