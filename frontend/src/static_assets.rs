use log::*;
use once_cell::sync::{
    Lazy,
    OnceCell,
};

pub static BASE_URL: Lazy<OnceCell<String>> = Lazy::new(|| {
    // TODO: configure base url from server
    let url = web_sys::window().unwrap().location().origin().unwrap();
    trace!("base url is {url}");
    let cell = OnceCell::new();
    cell.set(url).unwrap();
    cell
});
// TODO: setup lazy fetch for static images/themes
//
// probably with an /api/theme/ endpoint
