mod bridge_generated;

use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

mod api;

// i dunno why this many worker_threads
pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads((num_cpus::get_physical() / 2).min(1))
        .enable_all()
        .build()
        .unwrap()
});
pub static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());

#[derive(Debug, Default)]
pub struct MioClientState {
    url: String,
}

impl MioClientState {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn test_set_url(&mut self, url: String) -> anyhow::Result<()> {
        let ret = CLIENT
            .get(format!("{url}/ver"))
            .send()
            .await?
            .json::<mio_common::Vers>()
            .await?;
        if ret
            == (mio_common::Vers::new(
                konst::unwrap_ctx!(konst::primitive::parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
                konst::unwrap_ctx!(konst::primitive::parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
                konst::unwrap_ctx!(konst::primitive::parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
            ))
        {
            self.url = url;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "either the version is outdated or this is not a miogenes server"
            ))
        }
    }
}
