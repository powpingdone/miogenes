use crate::tasks;
use dioxus::prelude::*;
use dioxus_router::*;
use futures::*;
use uuid::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn Login(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    let rtr = use_router(cx);
    let username = use_state(cx, String::default);
    let password = use_state(cx, String::default);
    let err_str = use_ref(cx, String::default);
    let login_routine = use_coroutine(cx, |mut rx: UnboundedReceiver<(String, String)>| {
        let rtr = rtr.clone();
        let token = token.clone();
        let err_str = err_str.clone();
        async move {
            while let Some((user, pass)) = rx.next().await {
                match tasks::get_token(user, pass).await {
                    Ok(good) => {
                        token.set(Some(good.0));
                        rtr.navigate_to("/home");
                    }
                    Err(err) => err_str.set(err),
                }
            }
        }
    });
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
                r#type: "password",
                oninput: move |evt| {
                    password.set(evt.value.clone())
                },
            }
            div {
                input {
                    r#type: "button",
                    value: "Login",
                    onclick: move |_| {
                        login_routine.send((username.get().clone(), password.get().clone()))
                    },
                }
                input {
                    r#type: "button",
                    value: "Signup",
                    onclick: move |_| {
                        rtr.navigate_to("/signup")
                    },
                }
            }
            p {
                format!("{}", err_str.read())
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn Signup(cx: Scope) -> Element {
    let rtr = use_router(cx);
    let username = use_state(cx, String::default);
    let password = use_state(cx, String::default);
    let password_check = use_state(cx, String::default);
    let err_str = use_ref(cx, String::default);
    let signin_routine = use_coroutine(cx, |mut rx: UnboundedReceiver<(String, String)>| {
        let rtr = rtr.clone();
        let err_str = err_str.clone();
        async move {
            while let Some((user, pass)) = rx.next().await {
                match tasks::signup_send(user, pass).await {
                    Ok(_) => rtr.navigate_to("/"),
                    Err(err) => err_str.set(err),
                }
            }
        }
    });
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
                r#type: "password",
                oninput: move |evt| {
                    password.set(evt.value.clone())
                },
            }
            p {
                "Retype Password"
            }
            input {
                value: "{password_check}",
                r#type: "password",
                oninput: move |evt| {
                    password_check.set(evt.value.clone())
                },
            }
            div {
                input {
                    r#type: "button",
                    value: "Signup",
                    onclick: move |_| {
                        if password.current() == password_check.current() {
                            signin_routine.send((username.get().clone(), password.get().clone()))
                        } else {
                            err_str.set("passwords do not match".to_owned())
                        }
                    },
                }
            }
            p {
                format!("{}", err_str.read())
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    cx.render(rsx! {
        div {}
    })
}
