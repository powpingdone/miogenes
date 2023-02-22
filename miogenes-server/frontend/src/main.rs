use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlDocument;

mod mainpage;
mod routepts;
mod tasks;

fn app_main(cx: Scope) -> Element {
    let curr_token = use_ref(cx, || None::<Uuid>);

    // load token if set
    // this target_arch directive is only here for r-a
    // because r-a thinks this is a amd64 project
    #[cfg(target_arch = "wasm32")]
    if let Some(Ok(token)) = wasm_cookies::get("Token") {
        if let Some(token) = Uuid::parse_str(token) {
            curr_token.set(token);
        }
    }

    // app routes
    cx.render(rsx!{
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
    cx.render(rsx!{
        div {}
    })
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    dioxus_web::launch(app_main);
}
