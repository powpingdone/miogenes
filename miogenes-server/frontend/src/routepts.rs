use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn Login<'a>(cx: Scope, token: &'a UseRef<Option<Uuid>>) -> Element {
    let rtr = use_router(cx);
    let username = use_state(cx, String::default);
    let password = use_state(cx, String::default);
    cx.render(rsx! {
        div {
            p {
                "Username"
            }
            input {
                value: "{username}",
                oninput: move |evt| {
                    username.set(evt.value.clone())
                },
            }
            p {
                "Password"
            }
            input {
                value: "{password}",
                oninput: move |evt| {
                    password.set(evt.value.clone())
                },
            }
            div {
                input {
                    r#type: "button",
                    value: "Login",
                }
                input {
                    r#type: "button",
                    value: "Signup",
                    onclick: move |_| {
                        rtr.navigate_to("/signup")
                    },
                }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn Signup<'a>(cx: Scope, token: &'a UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx! {
        div {}
    })
}
