// NOTE: these _have_ to be re-exported as pub in order for rust<->dart to work
// properly
use crate::error::ErrorSplit;
pub use crate::MioClientState;
use anyhow::bail;
pub use flutter_rust_bridge::RustOpaque;
pub use flutter_rust_bridge::SyncReturn;
use mio_common::retstructs;
pub use std::sync::Arc;
pub use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MioClient(pub RustOpaque<Arc<RwLock<MioClientState>>>);

pub fn new_mio_client() -> SyncReturn<MioClient> {
    SyncReturn(MioClient(RustOpaque::new(Arc::new(RwLock::new(
        MioClientState::new(),
    )))))
}

impl MioClient {
    pub fn get_url(&self) -> SyncReturn<String> {
        SyncReturn(self.0.read().unwrap().url.clone())
    }

    pub fn test_set_url(&self, url: String) -> anyhow::Result<()> {
        let mut lock = self.0.write().unwrap();
        lock.test_set_url(url)
    }

    pub fn attempt_signup_and_login(
        &self,
        username: String,
        password: String,
        password2: String,
    ) -> anyhow::Result<()> {
        if username.is_empty()||password.is_empty() || password2.is_empty() {
            bail!("No field may be empty.");
        }
        if password != password2 {
            bail!("The passwords do not match.");
        }
        let lock = self.0.read().unwrap();
        if let Err(err) = lock.attempt_signup(&username, &password) {
            rewrap_error(err, |status, resp| match status {
                409 => bail!("{resp}"),
                _ => Ok((status, resp)),
            })
        } else {
            drop(lock);
            self.attempt_login(username, password)
        }
    }

    pub fn attempt_login(&self, username: String, password: String) -> anyhow::Result<()> {
        let mut lock = self.0.write().unwrap();
        if let Err(err) = lock.attempt_login(&username, &password) {
            rewrap_error(err, |status, resp| match status {
                401 => bail!("{resp}"),
                _ => Ok((status, resp)),
            })
        } else {
            Ok(())
        }
    }

    // wrap endpoints so that it can autorefresh tokens
    fn wrap_refresh<Callback, Ret>(&self, cb: Callback) -> anyhow::Result<Ret>
    where
        Callback: Fn(&MioClientState) -> anyhow::Result<Ret>,
    {
        let lock = self.0.read().unwrap();
        let jwt = lock.key.get();
        if let Some(inner) = jwt {
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

// turn server error into something for human consumption
fn rewrap_error<Callback, Ret>(err: ErrorSplit, cb: Callback) -> anyhow::Result<Ret>
where
    Callback: FnOnce(u16, String) -> anyhow::Result<(u16, String)>,
{
    match err {
        ErrorSplit::Ureq(resp) => match *resp {
            // any other error besides a "not OK" statuscode is what we're handling here
            ureq::Error::Status(status, resp) => {
                // extract _any_ string
                let resp_dump = resp
                    .into_string()
                    .map_err(|err| format!("Error could not be decoded: {err}"))
                    .and_then(|error_json| {
                        serde_json::from_str::<retstructs::ErrorMsg>(&error_json)
                            .map(|x| x.error)
                            .map_err(|err| format!("Error message could not be extracted: {err}. Original message: {error_json}"))
                    });

                // they're all sinners in the end. doesn't matter. merge 'em
                let resp_str = match resp_dump {
                    Ok(x) | Err(x) => x,
                };

                // actual handler
                match cb(status, resp_str) {
                    Err(err) => Err(err),
                    Ok((status, resp)) => match status {
                        500 => bail!("INTERNAL SERVER ERROR: {resp}"),
                        _ => bail!("The server returned an unexpected error code {status}: {resp}"),
                    },
                }
            }
            ureq::Error::Transport(transport) => Err(transport.into()),
        },
        ErrorSplit::Other(err) => Err(err),
    }
}
