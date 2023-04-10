use dioxus::{
    prelude::*,
};
use dioxus_router::*;
use mio_common::retstructs::Album;
use uuid::*;
use wasm_bindgen::{
    JsCast,
};
use web_sys::{
    HtmlInputElement,
};
use crate::mainpage::tasks::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    // TODO: on req fail, check if it's a failure related to auth, and if so, delete
    // the Token cookie and refresh the page
    cx.render(rsx!{
        Router {
            Route {
                to: "/",
                HomePage {}
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn HomePage(cx: Scope) -> Element {
    // semi hack: because we cannot pass a UseFuture to a coroutine (lifetime
    // shenatigans and whatnot), pass a tiny bit of state around to restart on change
    let reset_albums = use_state::<u8>(&cx, || 0);
    let fetch_albums = use_future(&cx, reset_albums, |_| fetch_albums());
    let server_upload = use_coroutine(&cx, |rx| upload_to_server(rx, reset_albums.to_owned()));
    cx.render(rsx!{
        p {
            div {
                hidden: true,
                reset_albums.to_string()
            }
            {
                match fetch_albums.value() {
                    Some(albums) => {
                        let albums = albums.iter().map(|x| rsx!{
                            CoverArt { album: x }
                        });

                        rsx!{
                            albums
                        }
                    },
                    None => rsx!{
                        div {}
                    },
                }
            }
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
pub fn CoverArt<'a>(cx: Scope, album: &'a Album) -> Element {
    // TODO: album with no tracks?
    //
    // TODO: albums can have multiple different cover arts
    //
    // TODO: blank image for no album art/not loaded
    let track_id = album.tracks[0];
    let fetch = use_future(cx, (), |()| async move {
        let cl = reqwest::Client::new();
        let ret =
            cl
                .get(format!("{}/api/query/ti?", crate::BASE_URL.get().unwrap()))
                .query(&mio_common::msgstructs::IdInfoQuery { id: track_id })
                .send()
                .await
                .unwrap()
                .json::<mio_common::retstructs::Track>()
                .await
                .unwrap();
        if let Some(id) = ret.cover_art {
            Some(
                format!(
                    "{}/api/query/ca?{}",
                    crate::BASE_URL.get().unwrap(),
                    serde_urlencoded::to_string(&mio_common::msgstructs::IdInfoQuery { id }).unwrap()
                ),
            )
        } else {
            None
        }
    });
    cx.render(rsx!{
        {
            match fetch.value() {
                Some(Some(url)) => {
                    rsx!{
                        img { src: url.as_str() }
                    }
                },
                _ => rsx!{
                    div {}
                },
            }
        }
    })
}
