use dioxus::prelude::*;
use dioxus_router::*;
use log::*;
use uuid::*;

mod mainpage;
mod routepts;
mod tasks;

#[inline_props]
fn app_main(cx: Scope, token: Option<Uuid>) -> Element {
    let curr_token = use_ref(cx, || *token);

    // app routes
    cx.render(rsx! {
        Router {
            {
                if curr_token.read().is_none() {
                    rsx!{
                        Route {
                            to: "/",
                            routepts::Login { token: curr_token.clone() }
                        }
                        Route {
                            to: "/signup",
                            routepts::Signup {}
                        }
                    }
                } else {
                    rsx!{
                        Route {
                            to: "/",
                            Redir {}
                        }
                        Route {
                            to: "/home",
                            mainpage::MainPage { token: curr_token.clone() }
                        }
                    }
                }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
fn Redir(cx: Scope) -> Element {
    let rtr = use_router(cx);
    rtr.navigate_to("/home");
    cx.render(rsx! {
        div {}
    })
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));

    // load token if set. parse out uuid from str
    //
    // this target_arch directive is only here for r-a because r-a thinks this is a amd64
    // project. this is similar for all of the stuff interacting with wasm_cookies
    #[cfg(not(target_arch = "wasm32"))]
    let token: Option<Uuid> = None;
    #[cfg(target_arch = "wasm32")]
    let token = match wasm_cookies::get("Token") {
        Some(Ok(token)) => match Uuid::parse_str(&token) {
            Ok(token) => Some(token),
            Err(err) => {
                debug!("Failed to parse out token: {err}");
                None
            }
        },
        Some(Err(err)) => {
            debug!("wasm_cookies decoding failed: {err}");
            None
        }
        None => {
            debug!("No token found.");
            None
        }
    };
    dioxus_web::launch_with_props(
        app_main,
        app_mainProps { token },
        dioxus_web::Config::default(),
    );
}
