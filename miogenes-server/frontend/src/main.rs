use crate::state::*;
use dioxus::prelude::*;

mod state;
mod tasks;

fn app_main(cx: Scope) -> Element {
    let curr_state = use_ref(cx, State::default);
    let curr_token = use_ref(cx, || None);
    cx.render(rsx!{
        div {
            StatePage {
                state: curr_state,
                token: curr_token,
            }
        }
    })
}

fn main() {
    dioxus_web::launch(app_main);
}
