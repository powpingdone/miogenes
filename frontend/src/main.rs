use error::MFResult;
use mio_glue::MioClientState;
use std::{future::Future, sync::Arc};
use tokio::sync::RwLock;

slint::include_modules!();

mod error;
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
}

// weak version to prevent refcycles
#[derive(Clone)]
pub(crate) struct MioFrontendWeak {
    state: std::sync::Weak<RwLock<MioClientState>>,
    app: slint::Weak<TopLevelWindow>,
    rt: std::sync::Weak<tokio::runtime::Runtime>,
}

impl MioFrontendStrong {
    pub fn new(
        state: Arc<RwLock<MioClientState>>,
        app: TopLevelWindow,
        rt: Arc<tokio::runtime::Runtime>,
    ) -> Self {
        MioFrontendStrong { state, app, rt }
    }

    pub fn weak(&self) -> MioFrontendWeak {
        MioFrontendWeak {
            state: Arc::downgrade(&self.state),
            app: self.app.as_weak(),
            rt: Arc::downgrade(&self.rt),
        }
    }

    pub fn run(&self) -> MFResult<()> {
        self.app.run().map_err(|err| err.into())
    }

    // scoped global function, where the global is used scoped for all
    #[must_use]
    pub fn scoped_global<'b, GB, T, Ret>(&'b self, gb_fn: T) -> MFResult<Ret>
    where
        T: Fn(GB) -> Ret,
        GB: slint::Global<'b, TopLevelWindow>,
    {
        let gb = self.app.global::<GB>();
        let ret = gb_fn(gb);
        Ok(ret)
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
    let rt = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap(),
    );
    let state = Arc::new(RwLock::new(MioClientState::new()));
    let app = TopLevelWindow::new().unwrap();
    let s_state = MioFrontendStrong::new(state, app, rt);
    let state = s_state.weak();

    // setup callbacks
    s_state
        .scoped_global::<LoginBoxCB, _, _>(|x| {
            x.on_check_url({
                let state = state.clone();
                move |url| state.check_url(url)
            });
            x.on_attempt_login({
                let state = state.clone();
                move |username, password| state.attempt_login(username, password)
            });
        })
        .unwrap();
    s_state
        .scoped_global::<SignupBoxCB, _, _>(|x| {
            x.on_attempt_signup({
                let state = state.clone();
                move |username, password, password2| {
                    state.attempt_signup(username, password, password2)
                }
            });
        })
        .unwrap();
    s_state.run().unwrap();
}
