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
#[allow(non_snake_case)]
pub fn StatePage<'a>(
    cx: Scope,
    state: &'a UseRef<State>,
    token: &'a UseRef<Option<Uuid>>,
) -> Element {
    cx.render(rsx! {
        div {
            // render state to page
            {
                match *state.write() {
                    State::Login => {

                        let username_buf = use_state(cx, String::default);
                        let password_buf = use_state(cx, String::default);

                        rsx!{
                            div {
                                p {
                                    "Username"
                                }
                                input {}
                                p {
                                    "Password"
                                }
                                input {}
                            }
                        }
                    },
                    State::Signup => rsx!{
                        div {}
                    },
                    State::Main { ref mut page } => rsx!{
                        div {}
                    },
                }
            }
        }
    })
}
