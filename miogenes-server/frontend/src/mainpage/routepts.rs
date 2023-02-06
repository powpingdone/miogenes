use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx! {
        Router {
            Route {
                to: "/"
                HomePage { token: token.clone() }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn HomePage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx! {
        div {

        }
    })
}
