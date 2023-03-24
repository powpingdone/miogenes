use dioxus::{
    prelude::*,
};
use dioxus_router::*;
use js_sys::{
    ArrayBuffer,
    Uint8Array,
};
use mio_common::*;
use uuid::*;
use wasm_bindgen::{
    prelude::*,
    JsCast,
};
use wasm_bindgen_futures::*;
use web_sys::{
    Blob,
    HtmlInputElement,
};
use reqwest::Client;
use log::*;
use crate::BASE_URL;

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx!{
        Router {
            Route {
                to: "/",
                HomePage { token: token.clone() }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn HomePage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    let fetch_albums = use_future(&cx, (token,), |(token,)| fetch_albums(token.read().unwrap()));
    cx.render(rsx!{
        p {
            format!("{:?}", fetch_albums.value())
        }
        input {
            r#type: "file",
            id: "inp",
            multiple: "false",
        }
        button {
            // TODO: move this into it's own function
            onclick: move | evt | {
                log::trace!("onclick");
                evt.stop_propagation();
                let files =
                    web_sys::window()
                        .unwrap()
                        .document()
                        .unwrap()
                        .get_element_by_id("inp")
                        .unwrap()
                        .dyn_ref::<HtmlInputElement>()
                        .unwrap()
                        .files()
                        .unwrap();
                for pos in 0 .. files.length() {
                    let file = files.item(pos).unwrap();
                    let fname = file.name();
                    let blob: Blob = file.into();
                    spawn_local({
                        let token = token.read().unwrap();
                        async move {
                            let blob =
                                Uint8Array::new(
                                    JsFuture::from(blob.array_buffer())
                                        .await
                                        .unwrap()
                                        .dyn_ref::<ArrayBuffer>()
                                        .unwrap(),
                                ).to_vec();
                            let client = Client::new();
                            let req =
                                client
                                    .put(BASE_URL.get().unwrap().to_owned() + "/api/track/tu")
                                    .query(&msgstructs::TrackUploadQuery { fname: if fname != "" {
                                        Some(fname)
                                    } else {
                                        None
                                    } })
                                    .header("Content-Length", &blob.len().to_string())
                                    .header("Content-Type", "application/octet-stream")
                                    .header("Authorization", &format!("Bearer {}", token))
                                    .body(blob)
                                    .send()
                                    .await;
                            log::trace!("{req:?}");
                        }
                    });
                    fetch_albums.restart();
                }
            },
            "Send over."
        }
    })
}

async fn fetch_albums(token: Uuid) -> Vec<retstructs::Album> {
    // fetch initial ids
    let client = Client::new();
    let req =
        client
            .get(BASE_URL.get().unwrap().to_owned() + "/api/load/albums")
            .bearer_auth(token)
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
                        .get(BASE_URL.get().unwrap().to_owned() + "/api/query/ai")
                        .query(&msgstructs::IdInfoQuery { id: uuid })
                        .bearer_auth(token)
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
