use dioxus::{
    prelude::*,
    html::input_data::MouseButton,
};
use dioxus_router::*;
use uuid::*;
use gloo_net::http::{
    Request,
};
use wasm_bindgen::{
    JsCast,
    prelude::*,
};
use web_sys::{
    Blob,
    FileReader,
    HtmlInputElement,
};
use wasm_bindgen_futures::*;
use std::sync::Arc;

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
    let fut = use_future(&cx, (token,), |(token,)| async move {
        Request::get(&format!("/api/load/albums"))
            .header("Authorization", &format!("Bearer {}", token.read().unwrap()))
            .send()
            .await
    });
    cx.render(rsx!{
        div {
            input {
                r#type: "file",
                id: "inp",
                // this does mark this as a multiple file upload
                multiple: "false",
            }
            button {
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
                        let reader = Arc::new(FileReader::new().unwrap());
                        let cl = Closure::once_into_js({
                            let reader = reader.clone();
                            move || {
                                spawn_local({
                                    async move {
                                        let req =
                                            Request::put(&format!("/track/tu?fname={fname}"))
                                                .body(reader.result().unwrap())
                                                .send()
                                                .await;
                                        log::trace!("{req:?}");
                                    }
                                });
                            }
                        });
                        reader.set_onload(Some(cl.as_ref().unchecked_ref()));
                        reader.read_as_binary_string(&blob).unwrap();
                    }
                    fut.restart();
                },
                "Send over."
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
