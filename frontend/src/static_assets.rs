use log::*;
use once_cell::sync::{
    Lazy,
};

// base url for the rest of the inputs
pub static BASE_URL: Lazy<String> = Lazy::new(|| {
    // TODO: configure base url from server
    let url = web_sys::window().unwrap().location().origin().unwrap();
    trace!("base url is {url}");
    url
});

// reqwest client asset
pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::new()
});
// TODO: setup lazy fetch for static images/themes
//
// probably with an /api/theme/ endpoint
