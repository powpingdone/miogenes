use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Default, PartialEq)]
pub enum State {
    #[default]
    Login,
    Signup,
    Main {
        page: Page,
    },
}

#[derive(Default, PartialEq)]
pub enum Page {
    #[default]
    Home,
}

#[inline_props]
pub fn StatePage<'a>(cx: Scope, state: &'a UseRef<State>, token: &'a Option<Uuid>) -> Element {
    cx.render(rsx!{
        div {
            // render state to page
            {
                match *state.read() {
                    State::Login => rsx!{
                        div {}
                    },
                    State::Signup => rsx!{
                        div {}
                    },
                    State::Main { ref page } => rsx!{
                        div {}
                    },
                }
            }
        }
    })
}
