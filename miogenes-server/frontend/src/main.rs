use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;

mod mainpage;
mod routepts;
mod tasks;

fn app_main(cx: Scope) -> Element {
    let curr_token = use_ref(cx, || None::<Uuid>);
    cx.render(rsx! {
        Router {
            Route {
                to: "/",
                routepts::Login { token: curr_token.clone() }
            }
            Route {
                to: "/signup",
                routepts::Signup {}
            }
            Route {
                to: "/home",
                mainpage::MainPage { token: curr_token.clone() }
            }
        }
    })
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    dioxus_web::launch(app_main);
}
