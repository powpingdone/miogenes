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
    cx.render(rsx!{
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
    })
}
