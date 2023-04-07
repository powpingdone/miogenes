use dioxus::{
    prelude::*,
};
use dioxus_router::*;
use mio_common::retstructs;
use reqwest::StatusCode;
use uuid::*;
use wasm_bindgen::{
    JsCast,
};
use web_sys::{
    HtmlInputElement,
};
use log::*;
use crate::mainpage::tasks::*;

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
    // semi hack: because we cannot pass a UseFuture to a coroutine (lifetime
    // shenatigans and whatnot), pass a tiny bit of state around to restart on change
    let reset_albums = use_state::<u8>(&cx, || 0);
    let fetch_albums = use_future(&cx, (token, reset_albums), |(token, _)| fetch_albums(token.read().unwrap()));
    let server_upload =
        use_coroutine(&cx, |rx| upload_to_server(rx, reset_albums.to_owned(), token.read().unwrap()));
    cx.render(rsx!{
        p {
            div {
                hidden: true,
                reset_albums.to_string()
            }
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
                server_upload.send((0 .. files.length()).map(|x| files.item(x).unwrap()).collect());
            },
            "Send over."
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn AlbumArt(cx: Scope, token: UseRef<Option<Uuid>>, cover_art: Uuid) -> Element {
    let fetch = use_future(&cx, (token, cover_art), |(token, cover_art)| async move {
        let cl = reqwest::Client::new();
        let resp =
            cl
                .get(crate::BASE_URL.get().unwrap().to_owned() + "/api/query/ca")
                .bearer_auth(token.read().unwrap())
                .query(&mio_common::msgstructs::IdInfoQuery { id: cover_art })
                .send()
                .await;
        match resp {
            Ok(resp) if resp.status() == StatusCode::OK => {
                Ok(resp.json::<retstructs::CoverArt>().await.unwrap().data)
            },
            Ok(resp) => {
                Err(todo!())
            },
            Err(err) => {
                Err(todo!())
            },
        }
    });
    None
}
