use dioxus::{
    prelude::*,
};
use dioxus_router::*;
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
                    server_upload.send(files.item(pos).unwrap());
                }
            },
            "Send over."
        }
    })
}
