use dioxus::{
    prelude::*,
};
use futures::StreamExt;
use js_sys::{
    ArrayBuffer,
    Uint8Array,
};
use mio_common::*;
use uuid::*;
use wasm_bindgen::{
    JsCast,
};
use wasm_bindgen_futures::*;
use web_sys::{
    Blob,
};
use reqwest::{
    Client,
    StatusCode,
};
use log::*;
use crate::BASE_URL;

pub async fn upload_to_server(
    mut rx: UnboundedReceiver<Vec<web_sys::File>>,
    restart_task: UseState<u8>,
) {
    let ls = tokio::task::LocalSet::new();
    loop {
        ls.run_until({
            async {
                let upload_tasks =
                    rx
                        .next()
                        .await
                        .unwrap()
                        .into_iter()
                        .map(
                            |file| (
                                file.name(),
                                tokio::task::spawn_local(upload_to_server_inner_task(file)),
                            ),
                        )
                        .collect::<Vec<_>>();
                for (fname, task) in upload_tasks.into_iter() {
                    match task.await {
                        Ok(Ok(code)) if code == StatusCode::OK => {
                            trace!("file {fname} uploaded sucessfully");
                        },
                        // TODO: be louder, put this on the DOM
                        Ok(Ok(code)) => {
                            error!("file {fname} failed to upload, server returned {code}");
                        },
                        Ok(Err(err)) => {
                            error!("internal service error: {err}");
                        },
                        Err(err) => {
                            error!("join error: {err}");
                        },
                    }
                }
            }
        }).await;

        // overflowing_add is used here to indicate that this _will_ overflow. this is
        // only used when doing debugging because debugging enables int overflow checking
        // otherwise, this optimizes out to the exact same thing as x + 1
        restart_task.modify(|x| x.overflowing_add(1).0);
    }
}

pub async fn upload_to_server_inner_task(file: web_sys::File) -> Result<StatusCode, anyhow::Error> {
    let fname = file.name();
    let blob: Blob = file.into();
    let blob =
        Uint8Array::new(
            JsFuture::from(blob.array_buffer()).await.unwrap().dyn_ref::<ArrayBuffer>().unwrap(),
        ).to_vec();
    let client = Client::new();
    let req =
        client
            .put(format!("{}/api/track/tu", BASE_URL.get().unwrap()))
            .query(&msgstructs::TrackUploadQuery { fname: if fname != "" {
                Some(fname)
            } else {
                None
            } })
            .header("Content-Length", &blob.len().to_string())
            .header("Content-Type", "application/octet-stream")
            .body(blob)
            .send()
            .await?;
    Ok(req.status())
}

pub async fn fetch_albums() -> Vec<retstructs::Album> {
    // TODO: caching
    //
    // fetch initial ids
    let client = Client::new();
    let req =
        client
            .get(format!("{}/api/load/albums", BASE_URL.get().unwrap()))
            .send()
            .await
            .unwrap()
            .json::<retstructs::Albums>()
            .await
            .unwrap();
    let ls = tokio::task::LocalSet::new();
    ls.run_until(async move {
        // then fetch the album metadatas
        let fetch = req.albums.into_iter().map(|uuid| {
            let client = client.clone();
            tokio::task::spawn_local(async move {
                let req =
                    client
                        .get(format!("{}/api/query/ai", BASE_URL.get().unwrap()))
                        .query(&msgstructs::IdInfoQuery { id: uuid })
                        .send()
                        .await;
                if let Err(err) = req {
                    error!("error fetching album {uuid}: {err:?}");
                    return None;
                }
                match req.unwrap().json::<retstructs::Album>().await {
                    Ok(ret) => Some(ret),
                    Err(err) => {
                        error!("error serializing album {uuid}: {err:?}");
                        None
                    },
                }
            })
        }).collect::<Vec<_>>();

        // finally, collect the albums
        let mut albums = vec![];
        for task in fetch {
            if let Ok(Some(ret)) = task.await {
                albums.push(ret);
            }
        }
        albums
    }).await
}
