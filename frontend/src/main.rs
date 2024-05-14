use error::MFResult;
use mio_glue::{
    player::{CurrentlyDecoding, DecoderMsg},
    MioClientState,
};
use slint::ComponentHandle;
pub use slint::Weak as SlWeak;
pub use std::sync::Weak as StdWeak;
use std::{future::Future, process::exit, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

slint::include_modules!();
mod albums;
mod error;
mod player;
mod user;
mod upload;

impl From<Uuid> for SlintUUID {
    fn from(value: Uuid) -> Self {
        SlintUUID {
            id: value.to_string().into(),
        }
    }
}

impl From<SlintUUID> for Uuid {
    fn from(value: SlintUUID) -> Self {
        // This really should _not_ fail. A slintUUID comes from a Uuid
        Uuid::from_str(&value.id).unwrap()
    }
}

// quick and dirty error msg function
impl<T, E> From<Result<T, E>> for ErrorInfo
where
    E: std::fmt::Display,
{
    fn from(value: Result<T, E>) -> Self {
        Self {
            is_error: value.is_err(),
            error: match value {
                Ok(_) => "".to_owned().into(),
                Err(err) => format!("{err}").into(),
            },
        }
    }
}

// global state that must be held across the whole program
pub(crate) struct MioFrontendStrong {
    rt: Arc<tokio::runtime::Runtime>,
    state: Arc<RwLock<MioClientState>>,
    app: TopLevelWindow,
    player_tx: crossbeam::channel::Sender<DecoderMsg>,
    player_rx: tokio::sync::watch::Receiver<CurrentlyDecoding>,
}

// weak version to prevent refcycles
#[derive(Clone)]
pub(crate) struct MioFrontendWeak {
    state: StdWeak<RwLock<MioClientState>>,
    app: SlWeak<TopLevelWindow>,
    rt: StdWeak<tokio::runtime::Runtime>,
    player_tx: crossbeam::channel::Sender<DecoderMsg>,
    player_rx: tokio::sync::watch::Receiver<CurrentlyDecoding>,
}

impl MioFrontendStrong {
    pub fn new(
        state: Arc<RwLock<MioClientState>>,
        app: TopLevelWindow,
        rt: tokio::runtime::Runtime,
        player_tx: crossbeam::channel::Sender<DecoderMsg>,
        player_rx: tokio::sync::watch::Receiver<CurrentlyDecoding>,
    ) -> Self {
        MioFrontendStrong {
            state: state.into(),
            app,
            rt: rt.into(),
            player_tx,
            player_rx,
        }
    }

    pub fn weak(&self) -> MioFrontendWeak {
        MioFrontendWeak {
            state: Arc::downgrade(&self.state),
            app: self.app.as_weak(),
            rt: Arc::downgrade(&self.rt),
            player_tx: self.player_tx.clone(),
            player_rx: self.player_rx.clone(),
        }
    }

    pub fn run(&self) -> MFResult<()> {
        self.app.run().map_err(|err| err.into())
    }

    // scoped global function, where the global is used scoped for all
    pub fn scoped_global<'b, GB, T, Ret>(&'b self, gb_fn: T) -> Ret
    where
        T: Fn(GB) -> Ret,
        GB: slint::Global<'b, TopLevelWindow>,
    {
        let gb = self.app.global::<GB>();
        gb_fn(gb)
    }
}

impl MioFrontendWeak {
    // generic helper functions for the weakrefs
    fn w_app(&self) -> MFResult<TopLevelWindow> {
        self.app.upgrade().ok_or(error::Error::StrongGoneApp)
    }

    #[allow(unused)]
    fn w_state(&self) -> MFResult<Arc<RwLock<MioClientState>>> {
        self.state.upgrade().ok_or(error::Error::StrongGoneState)
    }

    fn w_rt(&self) -> MFResult<Arc<tokio::runtime::Runtime>> {
        self.rt.upgrade().ok_or(error::Error::StrongGoneRuntime)
    }

    #[allow(unused)]
    fn w_player_tx(&self) -> MFResult<crossbeam::channel::Sender<DecoderMsg>> {
        Ok(self.player_tx.clone())
    }

    fn w_player_rx(&self) -> MFResult<tokio::sync::watch::Receiver<CurrentlyDecoding>> {
        Ok(self.player_rx.clone())
    }

    // callback spawner and error reporter
    #[must_use]
    fn cb_spawn<B>(&self, fut: B) -> MFResult<()>
    where
        B: Future<Output = MFResult<()>> + Send + 'static,
    {
        let w_app = self.app.clone();
        self.w_rt()?.spawn(async move {
            let ret = fut.await;

            // TODO: better error reporting, sorta like folke/noice.nvim
            if ret.is_err() {
                drop(w_app.upgrade_in_event_loop(|app| {
                    app.global::<GlobalError>().set_last_error(ret.into())
                }));
            }
        });
        Ok(())
    }
}

fn main() {
    // setup strong refs
    let rt = tokio::runtime::Builder::new_multi_thread()
        // try very minimal configuration
        .worker_threads(2)
        .max_blocking_threads(16)
        .enable_all()
        .build()
        .unwrap();
    let state = Arc::new(RwLock::new(MioClientState::new()));

    // TODO: make a more clear error message when the player cannot find a device
    let app = TopLevelWindow::new().unwrap();
    let (player_tx, player_rx) = player::start_player_hold_thread(state.clone(), &rt);
    let s_state = MioFrontendStrong::new(state, app, rt, player_tx, player_rx);
    let state = s_state.weak();

    // bg tasks
    state.start_player_poll_task();
    state.start_album_poll_task();

    // setup callbacks
    s_state.scoped_global::<LoginBoxCB, _, _>(|x| {
        x.on_check_url({
            let state = state.clone();
            move |url| state.check_url(url)
        });
        x.on_attempt_login({
            let state = state.clone();
            move |username, password| state.attempt_login(username, password)
        });
    });
    s_state.scoped_global::<SignupBoxCB, _, _>(|x| {
        x.on_attempt_signup({
            let state = state.clone();
            move |username, password, password2| state.attempt_signup(username, password, password2)
        });
    });
    s_state.scoped_global::<PlayerCB, _, _>(|x| {
        x.on_play({
            let state = state.clone();
            move || state.play()
        });
        x.on_pause({
            let state = state.clone();
            move || state.pause()
        });
        x.on_next({
            let state = state.clone();
            move || state.next()
        });
        x.on_prev({
            let state = state.clone();
            move || state.prev()
        });
        x.on_seek({
            let state = state.clone();
            move |pos| state.seek(pos)
        })
    });
    s_state.scoped_global::<AlbumsCB, _, _>(|x| {
        x.on_queue_track({
            let state = state.clone();
            move |album| state.queue(album.id.into())
        })
    });
    s_state.run().unwrap();

    // TODO: actually cleanup devices
    exit(0);
}
