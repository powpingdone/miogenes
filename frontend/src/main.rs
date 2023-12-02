use mio_glue::MioClientState;
use parking_lot::Mutex;
use std::sync::Arc;

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
                    app.global::<GlobalError>().set_last_error(ret.into());
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
                        app.global::<TopLevelCB>().set_page_t(TopLevelPage::Ready);
                    }
                    app.global::<GlobalError>().set_last_error(ret.into());
                })
                .unwrap();
        }
    });
    app.global::<SignupBoxCB>().on_attempt_signup({
        let int_state = state.clone();
        let w_app = w_app.clone();
        move |username, password, password2| {
            if password != password2 {
                if let Some(app) = w_app.upgrade() {
                    app.global::<GlobalError>().set_last_error(ErrorInfo {
                        is_error: true,
                        error: "passwords do not match".into(),
                    })
                }
                return;
            }
            let mut state = int_state.lock();
            let (username, password) = (String::from(username), String::from(password));
            let mut ret = state.attempt_signup(&username, &password);
            if ret.is_ok() {
                ret = state.attempt_login(&username, &password);
            }
            w_app
                .upgrade_in_event_loop(move |app| {
                    if ret.is_ok() {
                        app.global::<TopLevelCB>().set_page_t(TopLevelPage::Ready);
                    }
                    app.global::<GlobalError>().set_last_error(ret.into());
                })
                .unwrap()
        }
    });
    app.run().unwrap();
}
