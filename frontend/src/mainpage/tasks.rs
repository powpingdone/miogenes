use dioxus::{
    prelude::*,
};
use futures::StreamExt;
use js_sys::{
    ArrayBuffer,
    Uint8Array,
};
use mio_common::{
    *,
    retstructs::Album,
};
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
use crate::{
    BASE_URL,
    CLIENT,
};

pub async fn upload_to_server(mut rx: UnboundedReceiver<Vec<web_sys::File>>, restart_task: UseState<u64>) {
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
                        .map(|file| (file.name(), tokio::task::spawn_local(upload_to_server_inner_task(file))))
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
        restart_task.with_mut(|x| *x = x.overflowing_add(1).0);
    }
}

async fn upload_to_server_inner_task(file: web_sys::File) -> Result<StatusCode, anyhow::Error> {
    let fname = file.name();
    let blob: Blob = file.into();
    let blob =
        Uint8Array::new(
            JsFuture::from(blob.array_buffer()).await.unwrap().dyn_ref::<ArrayBuffer>().unwrap(),
        ).to_vec();
    let client = Client::new();
    let req =
        client
            .put(format!("{}/api/track/tu", *BASE_URL))
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

pub async fn fetch_albums(task: UseState<u64>) -> Result<(u64, Vec<Album>), anyhow::Error> {
    // TODO: caching
    //
    // fetch initial ids
    let req =
        CLIENT
            .get(format!("{}/api/load/albums", *BASE_URL))
            .send()
            .await?
            .json::<retstructs::Albums>()
            .await?;
    let ls = tokio::task::LocalSet::new();
    ls.run_until(async move {
        // then fetch the album metadatas
        let fetch = req.albums.into_iter().map(|uuid| {
            tokio::task::spawn_local(fetch_albums_inner(uuid))
        }).collect::<Vec<_>>();

        // finally, collect the albums
        let mut ret = vec![];
        for task in fetch {
            if let Some(item) = task.await?? {
                ret.push(item);
            }
        }
        Ok((*task.get(), ret))
    }).await
}

async fn fetch_albums_inner(id: Uuid) -> Result<Option<Album>, reqwest::Error> {
    let req =
        CLIENT
            .get(format!("{}/api/query/ai", *BASE_URL))
            .query(&msgstructs::IdInfoQuery { id })
            .send()
            .await;
    if let Err(err) = req {
        error!("error fetching album {id}: {err:?}");
        return Ok(None);
    }
    Ok(Some(req.unwrap().json::<retstructs::Album>().await?))
}

pub async fn album_track_fetch(album: Album) -> Result<Vec<retstructs::Track>, anyhow::Error> {
    let set = tokio::task::LocalSet::new();
    set.run_until(async move {
        let mut tasks = Vec::with_capacity(album.tracks.len());
        for track_id in album.tracks.iter().cloned() {
            tasks.push(tokio::task::spawn_local({
                album_track_fetch_inner(track_id)
            }));
        }
        let mut ret = Vec::with_capacity(album.tracks.len());
        for task in tasks {
            ret.push(task.await??);
        }
        Ok(ret)
    }).await
}

async fn album_track_fetch_inner(id: Uuid) -> Result<retstructs::Track, anyhow::Error> {
    Ok(
        CLIENT
            .get(format!("{}/api/query/ti", *BASE_URL))
            .query(&mio_common::msgstructs::IdInfoQuery { id })
            .send()
            .await?
            .json::<mio_common::retstructs::Track>()
            .await?,
    )
}
