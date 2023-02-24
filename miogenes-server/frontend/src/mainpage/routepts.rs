use dioxus::{
    prelude::*,
    html::input_data::MouseButton,
};
use dioxus_router::*;
use uuid::*;
use gloo_net::http::{
    Request,
};
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlInputElement,
    Blob,
};
use wasm_bindgen_futures::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx!{
        Router {
            Route {
                to: "/" HomePage {
                    token: token.clone()
                }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn HomePage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    let fut = use_future(&cx, (token,), |(token,)| async move {
        Request::get(&format!("/api/load/albums"))
            .header("Authorization", &format!("Bearer {}", token.read().unwrap()))
            .send()
            .await
    });
    cx.render(rsx!{
        div {
            form {
                input {
                    r#type: "file",
                    id: "inp",
                    // this does mark this as a multiple file upload
                    multiple: "false",
                }
                button {
                    onclick: move | _ | {
                        log::trace!("onclick");
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
                        let send_files = vec![];
                        for pos in 0 .. files.length() {
                            let file = files.item(pos).unwrap();
                            send_files.push((file.name(), {
                                let stream = AsRef::<Blob>::as_ref(&file).stream();
                                stream.tee()
                            }));
                        }
                        log::trace!("entering future.");
                        spawn_local({
                            async move {
                                log::trace!("entered future.");
                                log::trace!(
                                    "{:?}",
                                    Request::put(&format!("/track/tu")).body(send_files).send().await
                                );
                            }
                        });
                        fut.restart();
                    },
                    "Send over."
                }
            }
            div {
                {
                    match fut.value() {
                        Some(x) => {
                            format!("{x:?}")
                        },
                        None => {
                            "waiting...".to_owned()
                        },
                    }
                }
            }
        }
    })
}
