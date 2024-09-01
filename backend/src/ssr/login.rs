use dioxus::prelude::*;

pub fn LoginPage() -> Element {
    rsx! {
        // input form
        form {
            action: "/login",
            method: "post",
        }
    }
}
