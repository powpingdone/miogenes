use mio_glue::MioClientState;
use slint::SharedString;
use tokio::sync::RwLock;

use crate::*;

use slint::Weak as SlWeak;
use std::sync::Weak as StdWeak;

impl MioFrontendWeak {
    pub fn check_url(&self, url: SharedString) {
        self.cb_spawn(check_url_inner(
            self.state.clone(),
            self.app.clone(),
            url.into(),
        ))
        .unwrap();
    }

    pub fn attempt_login(&self, username: SharedString, password: SharedString) {
        self.cb_spawn(attempt_login_inner(
            self.state.clone(),
            self.app.clone(),
            username.into(),
            password.into(),
        ))
        .unwrap()
    }

    pub fn attempt_signup(
        &self,
        username: SharedString,
        password: SharedString,
        password2: SharedString,
    ) {
        self.cb_spawn(attempt_signin_inner(
            self.state.clone(),
            self.app.clone(),
            username.into(),
            password.into(),
            password2.into(),
        ))
        .unwrap()
    }
}

async fn check_url_inner(
    w_state: StdWeak<RwLock<MioClientState>>,
    w_app: SlWeak<TopLevelWindow>,
    url: String,
) -> MFResult<()> {
    let l_state = w_state
        .upgrade()
        .ok_or(crate::error::Error::StrongGoneState)?;
    let mut state = l_state.write().await;
    state.test_set_url(url.into()).await?;
    w_app.upgrade_in_event_loop(move |app| {
        app.global::<crate::LoginBoxCB>().set_url_is_valid(true);
    })?;
    Ok(())
}

async fn attempt_login_inner(
    w_state: StdWeak<RwLock<MioClientState>>,
    w_app: SlWeak<TopLevelWindow>,
    username: String,
    password: String,
) -> MFResult<()> {
    let l_state = w_state
        .upgrade()
        .ok_or(crate::error::Error::StrongGoneState)?;
    let mut state = l_state.write().await;
    state.attempt_login(&username, &password).await?;
    w_app.upgrade_in_event_loop(move |app| {
        app.global::<TopLevelCB>().set_page_t(TopLevelPage::Ready);
    })?;
    Ok(())
}

async fn attempt_signin_inner(
    w_state: StdWeak<RwLock<MioClientState>>,
    w_app: SlWeak<TopLevelWindow>,
    username: String,
    password: String,
    password2: String,
) -> MFResult<()> {
    let l_state = w_state
        .upgrade()
        .ok_or(crate::error::Error::StrongGoneState)?;
    let mut state = l_state.write().await;
    if password != password2 {
        return Err(error::Error::ClientSide(
            "passwords do not match".to_owned(),
        ));
    }
    state.attempt_signup(&username, &password).await?;
    state.attempt_login(&username, &password).await?;
    w_app.upgrade_in_event_loop(move |app| {
        app.global::<TopLevelCB>().set_page_t(TopLevelPage::Ready);
    })?;
    Ok(())
}
