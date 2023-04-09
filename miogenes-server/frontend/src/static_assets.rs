use log::*;
use std::sync::LazyLock;
use tokio::sync::{
    OnceCell as AsyncOnceCell,
};
use std::sync::OnceLock;
use uuid::Uuid;

pub static BASE_URL: LazyLock<String> = LazyLock::new(|| {
    // TODO: configure base url from server
    let url = web_sys::window().unwrap().location().origin().unwrap();
    trace!("base url is {url}");
    url
});

// TODO: setup lazy fetch for static images
//
// Struct to lazily fetch a static image.
struct _StaticImg {
    url: &'static str,
    cell: AsyncOnceCell<Vec<u8>>,
    auth: OnceLock<Uuid>,
}

impl _StaticImg {
    const fn new(url: &'static str) -> Self {
        Self {
            url,
            cell: AsyncOnceCell::const_new(),
            auth: OnceLock::new(),
        }
    }

    pub fn set_auth(&self, auth: Uuid) -> Result<(), Uuid> {
        self.auth.set(auth)
    }

    pub async fn fetch(&'static mut self) -> Result<&Vec<u8>, anyhow::Error> {
        self.cell.get_or_try_init(|| async {
            let cl = reqwest::Client::new();

            // TODO: figure out url for this
            //
            // TODO: auth?
            Ok(cl.get(BASE_URL.to_owned() + "/api/theme/wait").send().await?.bytes().await?.to_vec())
        }).await
    }
}
