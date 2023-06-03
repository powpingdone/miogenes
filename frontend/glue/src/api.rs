// NOTE: these _have_ to be re-exported as pub in order for rust<->dart to work
// properly
pub use crate::MioClientState;
pub use crate::RUNTIME;
pub use flutter_rust_bridge::RustOpaque;
pub use flutter_rust_bridge::SyncReturn;
pub use std::sync::RwLock;

#[derive(Debug)]
pub struct MioClient(pub RustOpaque<RwLock<MioClientState>>);

pub fn new_mio_client() -> SyncReturn<MioClient> {
    SyncReturn(MioClient(RustOpaque::new(RwLock::new(MioClientState::new()))))
}

impl MioClient {
    pub fn test_set_url(&self, url: String) -> anyhow::Result<()> {
        let mut lock = self.0.write().unwrap();
        RUNTIME.block_on(lock.test_set_url(url))
    }
}
