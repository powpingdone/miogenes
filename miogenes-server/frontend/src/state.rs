use egui::style::{DebugOptions};
use oneshot as osh;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::rt::RunTime;
use mio_common::*;

#[derive(Deserialize, Serialize, Default)]
pub enum Page {
    #[default]
    Loaded,
    #[serde(skip)]
    Login {
        user: String,
        pass: String,
        err_msg: Option<String>,
        login_resp: Option<osh::Receiver<Result<msgstructs::UserToken, String>>>,
    },
    Signup {
        #[serde(skip)]
        user: String,
        #[serde(skip)]
        pass: String,
        #[serde(skip)]
        pass2: String,
        #[serde(skip)]
        signup_resp: Option<osh::Receiver<Option<u16>>>,
        #[serde(skip)]
        err_msg: Option<String>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Application {
    pub page: Page,
    pub token: Option<Uuid>,
    #[serde(skip)]
    pub rt: RunTime,
    pub debug: DebugOptions,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            page: Page::default(),
            rt: RunTime::new(),
            token: Default::default(),
            debug: Default::default(),
        }
    }
}

impl Application {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // TODO: gui customization

        // egui persistence
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        use Page::*;

        if !frame.is_web() {
            panic!("run this application in a web browser");
        }

        match self.page {
            Loaded => {
                self.page = Login {
                    user: "".to_owned(),
                    pass: "".to_owned(),
                    err_msg: None,
                    login_resp: None,
                }
            }
            Login { .. } => {
                self.login_render(ctx, frame);
            }
            Signup { .. } => {
                self.signup_render(ctx, frame);
            }
        }
    }
}
