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
    let doc = web_sys::window().unwrap().document().unwrap();
    let htmdoc = doc.dyn_ref::<HtmlDocument>().unwrap();
    if let Ok(cookie) = htmdoc.cookie() {
        if cookie != "" {
            // parse out cookie
            let split = cookie.split(";");
            for x in split {
                log::trace!("cookie: {x}");
                let split_kv = x.split("=").collect::<Vec<_>>();
                let (k, v) = (split_kv[0], split_kv[1]);
                log::debug!("kv: (\"{k}\", \"{v}\")");
                if k == "Token" {
                    curr_token.set(Some(Uuid::parse_str(v).unwrap()));
                    break;
                }
            }
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
