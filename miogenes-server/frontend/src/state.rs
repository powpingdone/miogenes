use crate::rt::RunTime;
use egui::*;
use egui_extras::{Size, Strip, StripBuilder};
use log::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Default)]
pub enum Page {
    #[default]
    Loaded,
    #[serde(skip)]
    Login {
        user: String,
        pass: String,
        err_msg: Option<String>,
        login_resp: Option<oneshot::Receiver<Result<mio_common::msgstructs::UserToken, String>>>,
    },
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct Application {
    page: Page,
    token: Option<Uuid>,
    #[serde(skip)]
    rt: RunTime,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            page: Page::default(),
            rt: RunTime::new(),
            token: Default::default(),
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
        }
    }
}

impl Application {
    fn login_render(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Page::Login {
            ref mut user,
            ref mut pass,
            ref mut login_resp,
            ref mut err_msg,
        } = self.page
        {
            CentralPanel::default().show(ctx, |ui| {
                center_box(ui, [128.0, 52.0].into(), |ui: &mut Ui| {
                    Frame::none()
                        .stroke(Stroke::new(1.0, Color32::GRAY))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add(
                                    Label::new(RichText::new("Miogenes").size(25.0)).wrap(false),
                                );
                                ui.add(Label::new("Username: ").wrap(false));
                                let resp0 = ui.add(TextEdit::singleline(user));
                                ui.add(Label::new("Password: ").wrap(false));
                                let resp1 = ui.add(TextEdit::singleline(pass).password(true));

                                // send future on enter
                                if login_resp.is_none()
                                    && (resp0.lost_focus() || resp1.lost_focus())
                                    && ui.input().key_pressed(Key::Enter)
                                {
                                    *err_msg = None;
                                    let (tx, rx) = oneshot::channel();
                                    *login_resp = Some(rx);
                                    self.rt.push_future(get_token(
                                        tx,
                                        user.to_owned(),
                                        pass.to_owned(),
                                    ));
                                }

                                // check future
                                if let Some(ref mut rx) = *login_resp {
                                    use oneshot::TryRecvError;
                                    match rx.try_recv() {
                                        Ok(Ok(token)) => {
                                            debug!("token recieved: {}", token.0);
                                            self.token = Some(token.0)
                                        }
                                        Ok(Err(msg)) => {
                                            *err_msg = Some(msg);
                                        }
                                        Err(TryRecvError::Disconnected) => {
                                            if self.token.is_none() {
                                                *err_msg = Some(
                                                    "broken future/pipe encountered".to_owned(),
                                                );
                                            }
                                        }
                                        Err(TryRecvError::Empty) => (), // Do nothing
                                    }
                                }

                                // if future was a failure, don't poll again
                                if err_msg.is_some() {
                                    *login_resp = None;
                                }

                                ui.label({
                                    if let Some(ref msg) = *err_msg {
                                        msg
                                    } else {
                                        ""
                                    }
                                });
                            });
                        })
                        .response
                });
            });
        } else {
            unreachable!()
        }
    }
}

async fn get_token(
    tx: oneshot::Sender<Result<mio_common::msgstructs::UserToken, String>>,
    user: String,
    pass: String,
) -> Result<(), String> {
    use gloo_net::http::Request;

    let ret = Request::get("/l/login")
        .header(
            "Authorization",
            &format!("Basic {}", base64::encode(format!("{user}:{pass}"))),
        )
        .send()
        .await;

    let ret = {
        match ret {
            Ok(res) => {
                if res.ok() {
                    let ser = res.json().await;
                    match ser {
                        Ok(ret) => Ok(ret),
                        Err(err) => Err(format!("failed to seralize: {err}")),
                    }
                } else {
                    Err(format!(
                        "server returned err: {}, {}",
                        res.status(),
                        match res.body() {
                            None => "".to_owned(),
                            Some(body) => match body.to_string().as_string() {
                                None => "".to_owned(),
                                Some(ser) => ser,
                            },
                        }
                    ))
                }
            }
            Err(err) => Err(format!("failed to connect to server: {err}")),
        }
    };
    let chk = tx.send(ret.clone());
    if ret.is_ok() && chk.is_ok() {
        Ok(())
    } else if ret.is_err() {
        Err(ret.unwrap_err())
    } else {
        Err(chk.unwrap_err().to_string())
    }
}

// util function to do centered box
fn center_box(ui: &mut Ui, size: Vec2, f: impl Widget) {
    ui.vertical_centered(|ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::remainder().at_least(size.y))
            .size(Size::remainder())
            .vertical(|mut strip| {
                strip.empty();
                strip.cell(|ui| {
                    StripBuilder::new(ui)
                        .size(Size::remainder())
                        .size(Size::remainder().at_least(size.x))
                        .size(Size::remainder())
                        .horizontal(|mut strip| {
                            strip.empty();
                            strip.cell(|ui| {
                                ui.add(f);
                            });
                            strip.empty();
                        });
                });
                strip.empty();
            });
    });
}
