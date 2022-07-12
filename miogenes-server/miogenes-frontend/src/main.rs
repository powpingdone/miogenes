use dioxus::prelude::*;

fn main() {
    dioxus::web::launch(app);
}

fn app(cx: Scope) -> Element {
    
    cx.render(rsx!{
        div {
            onclick: move |_| {},
            p { "hello, wasm!" }
        }
    })
}
