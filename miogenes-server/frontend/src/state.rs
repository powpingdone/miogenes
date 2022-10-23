use crate::rt::RunTime;
use egui::*;
use log::*;
use serde::{Deserialize, Serialize};

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
            CentralPanel::default().show(ctx, |ui| {
                let mut size = ui.max_rect().size();
                size.x *= 0.15;
                size.y *= 0.05;
                let post = Rect::from_center_size(ui.max_rect().center(), size);

                ui.put(post, |ui: &mut Ui| {
                    Frame::none()
                        .stroke(Stroke::new(1.0, Color32::GRAY))
                        .inner_margin(style::Margin::symmetric(
                            0.1 * ui.max_rect().size().x,
                            0.15 * ui.max_rect().size().y,
                        ))
                        .show(ui, |ui: &mut Ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.125 * ui.max_rect().size().x;
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Miogenes").size(25.0));
                                });
                                ui.add(|ui: &mut Ui| {
                                    ui.vertical(|ui| {
                                        ui.add(Label::new("Username: ").wrap(false));
                                        ui.add(TextEdit::singleline(user));
                                        ui.add(Label::new("Password: ").wrap(false));
                                        ui.add(TextEdit::singleline(pass).password(true));
                                    })
                                    .response
                                });
                            })
                            .response
                        })
                        .response
                })
            });
        } else {
            unreachable!()
        }
    }
}
