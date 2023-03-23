use dioxus::{
    prelude::*,
};
use dioxus_router::*;
use gloo_net::http::Request;
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
        let r =
            Request::get(&format!("/api/load/albums"))
                .header("Authorization", &format!("Bearer {}", token.read().unwrap()))
                .send()
                .await;
        if let Ok(ret) = r {
            format!("{:?}", ret.json::<retstructs::Albums>().await)
        } else {
            format!("{r:?}")
        }
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
                                    );
                                let req =
                                    Request::put(
                                        &format!(
                                            "/api/track/tu?{}",
                                            serde_urlencoded::to_string(
                                                msgstructs::TrackUploadQuery { fname: if fname != "" {
                                                    Some(fname)
                                                } else {
                                                    None
                                                } },
                                            ).unwrap()
                                        ),
                                    )
                                        .header("Content-Length", &blob.length().to_string())
                                        .header("Content-Type", "application/octet-stream")
                                        .header("Authorization", &format!("Bearer {}", token))
                                        .body(blob)
                                        .send()
                                        .await;
                                log::trace!("{req:?}");
                            }
                        });
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
