use error::MFResult;
use mio_glue::{player::Player, MioClientState};
use std::{future::Future, sync::Arc};
use tokio::sync::RwLock;

pub use slint::Weak as SlWeak;
pub use std::sync::Weak as StdWeak;

slint::include_modules!();

mod error;
mod player;
mod user;

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
    state: Arc<RwLock<MioClientState>>,
    app: TopLevelWindow,
    rt: Arc<tokio::runtime::Runtime>,
    player: Arc<Player>,
}

// weak version to prevent refcycles
#[derive(Clone)]
pub(crate) struct MioFrontendWeak {
    state: StdWeak<RwLock<MioClientState>>,
    app: SlWeak<TopLevelWindow>,
    rt: StdWeak<tokio::runtime::Runtime>,
    player: StdWeak<Player>,
}

impl MioFrontendStrong {
    pub fn new(
        state: Arc<RwLock<MioClientState>>,
        app: TopLevelWindow,
        rt: tokio::runtime::Runtime,
        player: Player,
    ) -> Self {
        MioFrontendStrong {
            state: state.into(),
            app,
            rt: rt.into(),
            player: player.into(),
        }
    }

    pub fn weak(&self) -> MioFrontendWeak {
        MioFrontendWeak {
            state: Arc::downgrade(&self.state),
            app: self.app.as_weak(),
            rt: Arc::downgrade(&self.rt),
            player: Arc::downgrade(&self.player),
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

    fn w_state(&self) -> MFResult<Arc<RwLock<MioClientState>>> {
        self.state.upgrade().ok_or(error::Error::StrongGoneState)
    }

    fn w_rt(&self) -> MFResult<Arc<tokio::runtime::Runtime>> {
        self.rt.upgrade().ok_or(error::Error::StrongGoneRuntime)
    }

    fn w_player(&self) -> MFResult<Arc<Player>> {
        self.player.upgrade().ok_or(error::Error::StrongGonePlayer)
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
    let rt = 
        tokio::runtime::Builder::new_multi_thread()
            // try very minimal configuration
            .worker_threads(1)
            .max_blocking_threads(4)
            .enable_all()
            .build()
            .unwrap()
    ;
    let state = Arc::new(RwLock::new(MioClientState::new()));
    // TODO: make a more clear error message when the player cannot find a device
    let player = Player::new(state.clone()).unwrap();
    let app = TopLevelWindow::new().unwrap();
    let s_state = MioFrontendStrong::new(state, app, rt, player);
    let state = s_state.weak();

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
    s_state.run().unwrap();
}
