use std::sync::Arc;

use mio_glue::MioClientState;
use parking_lot::Mutex;

slint::include_modules!();

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

fn main() {
    let state = Arc::new(Mutex::new(MioClientState::new()));
    let app = TopLevelWindow::new().unwrap();
    let w_app = app.as_weak();

    // setup callbacks
    let x = app.global::<LoginBoxCB>();
    x.on_check_url({
        let int_state = state.clone();
        let w_app = w_app.clone();
        move |url| {
            let mut state = int_state.lock();
            let ret = state.test_set_url(url.into());
            w_app
                .upgrade_in_event_loop(move |app| {
                    let x = app.global::<LoginBoxCB>();
                    match ret {
                        Ok(()) => x.set_url_is_valid(true),
                        Err(_) => x.set_url_is_valid(false),
                    }
                    x.set_error(ret.into());
                })
                .unwrap();
        }
    });
    x.on_attempt_login({
        let int_state = state.clone();
        let w_app = w_app.clone();
        move |username, password| {
            let mut state = int_state.lock();
            let ret = state.attempt_login(&String::from(username), &String::from(password));
            w_app
                .upgrade_in_event_loop(move |app| {
                    if ret.is_ok() {
                        app.set_page_t(TopLevelPage::Ready);
                    }
                    app.global::<LoginBoxCB>().set_error(ret.into());
                })
                .unwrap();
        }
    });

    app.run().unwrap();
}
