use dioxus::prelude::*;
use dioxus_router::*;
use uuid::*;

mod routepts;
mod tasks;

fn app_main(cx: Scope) -> Element {
    let curr_token = use_ref(cx, || None::<Uuid>);
    cx.render(rsx!{
        Router {
            Route {
                to: "/",
                routepts::Login { token: curr_token }
            }
            Route {
                to: "/signup",
                routepts::Signup { token: curr_token }
            }
        }
    })
}

fn main() {
    dioxus_web::launch(app_main);
}
