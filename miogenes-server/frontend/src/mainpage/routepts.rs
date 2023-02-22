use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;
use gloo_net::http::{
    FormData,
    Request,
};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
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
                    r#type: "submit",
                    onsubmit: move | _ | {
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
                        let send_files = FormData::new().unwrap();
                        for pos in 0 .. files.length() {
                            let file = files.item(pos).unwrap();
                            send_files.append_with_blob(&format!("file{pos}"), file.as_ref()).unwrap();
                        }
                        spawn_local({
                            async move {
                                log::trace!("{:?}", Request::put("/track/tu").body(send_files).send().await);
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
