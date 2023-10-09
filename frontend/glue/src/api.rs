// NOTE: these _have_ to be re-exported as pub in order for rust<->dart to work
// properly
use crate::error::ErrorSplit;
pub use crate::player::Player;
use crate::player::PlayerMsg;
pub use crate::MioClientState;
use anyhow::bail;
pub use flutter_rust_bridge::RustOpaque;
use flutter_rust_bridge::StreamSink;
pub use flutter_rust_bridge::SyncReturn;
use log::*;
pub use mio_common::*;
pub use std::sync::Arc;
use std::sync::Once;
pub use std::sync::RwLock;
use std::{path::Path, time::Duration};
use uuid::Uuid;

pub fn init_self() -> SyncReturn<()> {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        #[cfg(target_os = "android")]
        {
            use android_logger::Config;

            // init logger
            android_logger::init_once(Config::default().with_max_level(LevelFilter::Trace));
        }
        #[cfg(not(target_os = "android"))]
        {
            // init logger
            drop(env_logger::try_init());
        }
    });
    SyncReturn(())
}

#[derive(Debug, Clone)]
pub struct MioClient(pub RustOpaque<Arc<RwLock<MioClientState>>>);

pub fn new_mio_client() -> SyncReturn<MioClient> {
    trace!("creating mio client");
    SyncReturn(MioClient(RustOpaque::new(Arc::new(RwLock::new(
        MioClientState::new(),
    )))))
}

#[derive(Clone, PartialEq, Eq)]
pub enum DecoderStatus {
    Playing,
    Paused,
    Buffering,
    Loading,
    Dead,
}

#[derive(Clone)]
pub struct PStatus {
    pub err_msg: Option<String>,
    pub queue: Vec<Uuid>,
    pub status: Option<DecoderStatus>,
    pub curr_playing: Option<Uuid>,
    pub playback_pos_s: u64,
    pub playback_pos_ms: u32,
    pub playback_len_s: u64,
    pub playback_len_ms: u32,
}

pub struct MioPlayer(pub RustOpaque<Player>);

pub fn new_player(client: MioClient) -> SyncReturn<MioPlayer> {
    trace!("creating mio player");
    SyncReturn(MioPlayer(RustOpaque::new(Player::new(Arc::clone({
        let x: &Arc<_> = &client.0;
        x
    })))))
}

impl MioPlayer {
    pub fn info_stream(&self, x: StreamSink<PStatus>) {
        trace!("sending sink");
        drop(self.0.tx.send(PlayerMsg::SetSink(x)));
    }

    pub fn play(&self, id: Option<Uuid>) {
        debug!("requesting play {id:?}");
        self.0.tx.send(PlayerMsg::Play(id)).unwrap();
    }

    pub fn pause(&self) {
        debug!("requesting pause");
        self.0.tx.send(PlayerMsg::Pause).unwrap();
    }

    pub fn toggle(&self) {
        debug!("requesting toggle");
        self.0.tx.send(PlayerMsg::Toggle).unwrap();
    }

    pub fn queue(&self, id: Uuid) {
        debug!("requesting queue {id:}");
        self.0.tx.send(PlayerMsg::Queue(id)).unwrap();
    }

    pub fn stop(&self) {
        debug!("requesting stop");
        self.0.tx.send(PlayerMsg::Stop).unwrap();
    }

    pub fn forward(&self) {
        debug!("requesting forward");
        self.0.tx.send(PlayerMsg::Forward).unwrap();
    }

    pub fn backward(&self) {
        debug!("requesting backward");
        self.0.tx.send(PlayerMsg::Backward).unwrap();
    }

    pub fn seek(&self, ms: u64) {
        let ms = Duration::from_millis(ms);
        debug!("requesting seek {ms:?}");
        self.0.tx.send(PlayerMsg::Seek(ms)).unwrap();
    }
}

impl MioClient {
    pub fn get_url(&self) -> SyncReturn<String> {
        SyncReturn(self.0.read().unwrap().url.clone())
    }

    pub fn test_set_url(&self, url: String) -> anyhow::Result<()> {
        trace!("testing url {url}");
        let mut lock = self.0.write().unwrap();
        lock.test_set_url(url)
    }

    pub fn attempt_signup_and_login(
        &self,
        username: String,
        password: String,
        password2: String,
    ) -> anyhow::Result<()> {
        if username.is_empty() || password.is_empty() || password2.is_empty() {
            bail!("No field may be empty.");
        }
        if password != password2 {
            bail!("The passwords do not match.");
        }
        let lock = self.0.read().unwrap();
        if let Err(err) = lock.attempt_signup(&username, &password) {
            rewrap_error(err, |status, resp| match status {
                403 | 409 => bail!("{resp}"),
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

    pub fn get_albums(&self) -> anyhow::Result<retstructs::Albums> {
        self.wrap_refresh(|lock| match lock.fetch_all_albums() {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_album(&self, id: Uuid) -> anyhow::Result<retstructs::Album> {
        self.wrap_refresh(|lock| match lock.get_album_data(id) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_track(&self, id: Uuid) -> anyhow::Result<retstructs::Track> {
        self.wrap_refresh(|lock| match lock.get_track_data(id) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_artist(&self, id: Uuid) -> anyhow::Result<retstructs::Artist> {
        self.wrap_refresh(|lock| match lock.get_artist_data(id) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_cover_art(&self, id: Uuid) -> anyhow::Result<retstructs::CoverArt> {
        self.wrap_refresh(|lock| match lock.get_cover_art_data(id) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_closest_track(
        &self,
        id: Uuid,
        ignore_tracks: Vec<Uuid>,
    ) -> anyhow::Result<retstructs::ClosestId> {
        self.wrap_refresh(|lock| match lock.get_closest(id, ignore_tracks.clone()) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| match status {
                404 => bail!("{resp}"),
                _ => Ok((status, resp)),
            }),
        })
    }

    pub fn get_files_at_dir(&self, path: String) -> anyhow::Result<Vec<String>> {
        let lock = self.0.read().unwrap();
        match lock.search_folder(path) {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| Ok((status, resp))),
        }
    }

    pub fn upload_file(
        &self,
        fullpath: String,
        dir: String,
    ) -> anyhow::Result<retstructs::UploadReturn> {
        let path = Path::new(&fullpath);
        let fname = path.file_name().map(|x| x.to_string_lossy().to_string());
        self.wrap_refresh(move |lock| {
            match lock.upload_file(fullpath.clone(), dir.to_owned(), fname.to_owned()) {
                Ok(ok) => Ok(ok),
                Err(err) => rewrap_error(err, |status, resp| match status {
                    400 | 409 => bail!("{resp}"),
                    _ => Ok((status, resp)),
                }),
            }
        })
    }

    pub fn get_folders(&self) -> anyhow::Result<Vec<crate::server::folder::FakeMapItem>> {
        self.wrap_refresh(|lock| match lock.get_folders() {
            Ok(ok) => Ok(ok),
            Err(err) => rewrap_error(err, |status, resp| Ok((status, resp))),
        })
    }

    pub fn make_dir(&self, name: String, path: String) -> anyhow::Result<()> {
        self.wrap_refresh(
            |lock| match lock.make_dir(name.to_owned(), path.to_owned()) {
                Ok(ok) => Ok(ok),
                Err(err) => rewrap_error(err, |status, resp| match status {
                    400 | 409 => bail!("{resp}"),
                    _ => Ok((status, resp)),
                }),
            },
        )
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
                        debug!("refreshing token");

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
            // any other error besides a "OK" statuscode is what we're handling here
            ureq::Error::Status(status, resp) => {
                // extract _any_ string
                let resp_dump =
                    resp
                        .into_string()
                        .map_err(|err| format!("Error could not be decoded: {err}"))
                        .and_then(|error_json| {
                            serde_json::from_str::<retstructs::ErrorMsg>(&error_json)
                                .map(|x| x.error)
                                .map_err(
                                    |err| format!(
                                        "Error message could not be extracted: {err}. Original message: {error_json}"
                                    ),
                                )
                        });

                // they're all sinners in the end. doesn't matter. merge 'em
                let resp_str = match resp_dump {
                    Ok(x) | Err(x) => x,
                };

                // actual handler
                match cb(status, resp_str) {
                    Err(err) => Err(err),
                    Ok((status, resp)) => match status {
                        401 => bail!("The key failed to be used, please re-login: {resp}"),
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
