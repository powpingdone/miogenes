use sea_orm::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::oneshot::channel as oneshot;

pub async fn track_upload_server(
    db: Arc<DatabaseConnection>,
    mut rx: UnboundedReceiver<(Uuid, Uuid, String)>,
) {
    let (tx_gc, mut rx_gc) = unbounded_channel();

    let gc = tokio::spawn(async move {
        // let mut queue = vec![];
        // tokio::select! ...
    });

    while let Some((id, userid, _orig_filename)) = rx.recv().await {
        tx_gc.send(tokio::spawn({
            let db = db.clone();
            async move {}
        }));
    }
}
