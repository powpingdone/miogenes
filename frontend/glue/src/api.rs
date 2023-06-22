// NOTE: these _have_ to be re-exported as pub in order for rust<->dart to work
// properly
pub use crate::MioClientState;
pub use flutter_rust_bridge::RustOpaque;
pub use flutter_rust_bridge::SyncReturn;
pub use std::sync::Arc;
pub use std::sync::RwLock;

use crate::ErrorSplit;
use anyhow::bail;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub struct MioClient(pub RustOpaque<Arc<RwLock<MioClientState>>>);

pub fn new_mio_client() -> SyncReturn<MioClient> {
    SyncReturn(MioClient(RustOpaque::new(Arc::new(RwLock::new(
        MioClientState::new(),
    )))))
}

impl MioClient {
    pub fn test_set_url(&self, url: String) -> anyhow::Result<()> {
        let mut lock = self.0.write().unwrap();
        lock.test_set_url(url)
    }

    // wrap endpoints so that it can autorefresh tokens
    fn wrap_refresh<Callback, Ret>(&self, cb: Callback) -> anyhow::Result<Ret>
    where
        Callback: Fn(&MioClientState) -> anyhow::Result<Ret>,
    {
        let lock = self.0.read().unwrap();
        let jwt = lock.key.read().unwrap();
        if let Some(ref inner) = *jwt {
            match inner.whois() {
                Ok(mdata) => {
                    // compare timestamp
                    if mdata.exp
                        < chrono::Utc::now()
                            .checked_add_signed(chrono::Duration::hours(12))
                            .unwrap()
                            .timestamp()
                    {
                        // refresh because it will be less than 12 hours to expiration
                        drop(jwt);
                        drop(lock);
                        let mut hold = self.0.write().unwrap();
                        hold.refresh_token()?;
                    }
                }
                Err(err) => bail!("could not decode token on the clientside: {err}"),
            }
        }
        let lock = self.0.read().unwrap();
        cb(&lock)
    }
}
