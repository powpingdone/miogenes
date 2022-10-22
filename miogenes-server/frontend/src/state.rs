use serde::{Deserialize, Serialize};

use crate::rt::RunTime;

#[derive(Deserialize, Serialize, Default)]
pub enum Page {
    #[default]
    Loaded,
    Login {
        #[serde(skip)]
        user: String,
        #[serde(skip)]
        pass: String,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Application {
    page: Page,
    #[serde(skip)]
    rt: RunTime,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            page: Page::default(),
            rt: RunTime::new(),
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
                }
            }
            Login { .. } => {
                self.login_render(ctx, frame);
            }
        }
    }
}

impl Application {
    fn login_render(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Page::Login {
            ref mut user,
            ref mut pass,
        } = self.page
        {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(user));
                ui.add(egui::TextEdit::singleline(pass).password(true))
            });
        } else {
            unreachable!()
        }
    }
}
