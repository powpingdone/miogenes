use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;

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
                    onsubmit: move |_| {
                        // send over files
                        // web_sys::FormData, append_with_blob 
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
