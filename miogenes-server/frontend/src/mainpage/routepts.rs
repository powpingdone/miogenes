use std::time::Duration;
use base64::{
    engine::{
        GeneralPurposeConfig,
        GeneralPurpose,
    },
    alphabet::STANDARD,
    Engine,
};
use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{
    JsFuture,
    spawn_local,
};
use web_sys::HtmlInputElement;

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
        gloo_net::http::Request::get(&format!("/api/load/albums"))
            .header(
                "Authorization",
                &format!(
                    "Bearer {}",
                    GeneralPurpose::new(
                        &STANDARD,
                        GeneralPurposeConfig::new(),
                    ).encode(format!("{}", token.read().unwrap()))
                ),
            )
            .send()
            .await
    });
    let fileconts = use_state(cx, String::default);
    cx.render(rsx!{
        div {
            input {
                r#type: "file",
                id: "inp",
                multiple: "false",
                onchange: |_| {
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
                    let mut send_files = vec![];
                    for pos in 0 .. files.length() {
                        let file = files.item(pos).unwrap();
                        send_files.push(JsFuture::from(file.text()));
                    }
                    spawn_local({
                        let fileconts = fileconts.clone();
                        async move {
                            let mut buf = "".to_owned();
                            for x in send_files {
                                buf += &x.await.unwrap().as_string().unwrap();
                            }
                            fileconts.set(buf);
                        }
                    });
                },
            }
            div {
                "{fileconts}"
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
