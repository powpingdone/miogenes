use egui::*;
use egui_extras::*;
use log::*;

use crate::state::*;
use crate::tasks;

// debug macro 
macro_rules! enable_debug {
    () => {
        self.debug.ui(ui);
        ui.style_mut().debug = self.debug;
    };
}

// util function to do centered box
fn center_box(ui: &mut Ui, size: Vec2, f: impl Widget) {
    ui.vertical_centered(|ui| {
        StripBuilder::new(ui)
            .size(Size::remainder())
            .size(Size::remainder().at_least(size.x))
            .size(Size::remainder())
            .horizontal(|mut strip| {
                strip.empty();
                strip.cell(|ui| {
                    StripBuilder::new(ui)
                        .size(Size::remainder())
                        .size(Size::remainder().at_least(size.y))
                        .size(Size::remainder())
                        .vertical(|mut strip| {
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

impl Application {
    pub fn login_render(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut signup_switch = false;
        let ref_signup_switch = &mut signup_switch;

        if let Page::Login {
            ref mut user,
            ref mut pass,
            ref mut login_resp,
            ref mut err_msg,
        } = self.page
        {
            CentralPanel::default().show(ctx, |ui| {
                center_box(ui, [96.0, 42.0].into(), |ui: &mut Ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(Label::new(RichText::new("Miogenes").size(25.0)).wrap(false));
                        ui.add(Label::new("Username: ").wrap(false));
                        let resp0 = ui.add(TextEdit::singleline(user));
                        ui.add(Label::new("Password: ").wrap(false));
                        let resp1 = ui.add(TextEdit::singleline(pass).password(true));

                        let mut signup_clicked = false;
                        let mut login_clicked = false;
                        let rs_c = &mut signup_clicked;
                        let rl_c = &mut login_clicked;
                        ui.vertical_centered(|ui| {
                            *rs_c = ui.button("Signup").clicked();
                            *rl_c = ui.button("Login").clicked();
                        });

                        // send future on enter or login press
                        if (login_resp.is_none()
                            && (resp0.lost_focus() || resp1.lost_focus())
                            && ui.input().key_pressed(Key::Enter))
                            || login_clicked
                        {
                            *err_msg = None;
                            let (tx, rx) = oneshot::channel();
                            *login_resp = Some(rx);
                            self.rt.push_future(tasks::get_token(
                                tx,
                                user.to_owned(),
                                pass.to_owned(),
                            ));
                        }

                        if signup_clicked {
                            *ref_signup_switch = true;
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
                                        *err_msg =
                                            Some("broken future/pipe encountered".to_owned());
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
                    })
                    .response
                });
            });
        } else {
            unreachable!()
        }

        if signup_switch {
            self.page = Page::Signup {
                user: "".to_owned(),
                pass: "".to_owned(),
                pass2: "".to_owned(),
                signup_resp: None,
                err_msg: None,
            };
        }
    }

    pub fn signup_render(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut successful = false;
        let inner_success = &mut successful;

        if let Page::Signup {
            ref mut user,
            ref mut pass,
            ref mut pass2,
            ref mut signup_resp,
            ref mut err_msg,
        } = self.page
        {
            CentralPanel::default().show(ctx, |ui| {
                center_box(ui, [96.0, 48.0].into(), |ui: &mut Ui| {
                    ui.vertical_centered(|ui| {
                        ui.add(
                            Label::new(RichText::new("Signup for Miogenes").size(20.0)).wrap(false),
                        );

                        ui.add(Label::new("Username").wrap(false));
                        let resp_user = ui.add(TextEdit::singleline(user));
                        ui.add(Label::new("Password").wrap(false));
                        let resp_pass0 = ui.add(TextEdit::singleline(pass).password(true));
                        ui.add(Label::new("Retype Password").wrap(false));
                        let resp_pass1 = ui.add(TextEdit::singleline(pass2).password(true));

                        // send future on enter
                        if signup_resp.is_none()
                            && (resp_user.lost_focus()
                                || resp_pass0.lost_focus()
                                || resp_pass1.lost_focus())
                            && ui.input().key_pressed(Key::Enter)
                        {
                            if pass != pass2 {
                                *err_msg = Some("Passwords do not match.".to_owned());
                            } else {
                                *err_msg = None;
                                let (tx, rx) = oneshot::channel();
                                *signup_resp = Some(rx);
                                self.rt.push_future(tasks::signup_send(
                                    tx,
                                    user.to_owned(),
                                    pass.to_owned(),
                                ));
                            }
                        }

                        if let Some(ref mut rx) = *signup_resp {
                            use oneshot::TryRecvError;
                            match rx.try_recv() {
                                Ok(Some(resp)) => {
                                    if resp == 200 {
                                        *inner_success = true;
                                    } else {
                                        *err_msg = Some(
                                            match resp {
                                                409 => "There is already a user with that name.",
                                                500 => "The server had a unhandleable error.",
                                                _e => {
                                                    error!("unknown response: {_e}");
                                                    "An unknown err code was returned."
                                                }
                                            }
                                            .to_owned(),
                                        );
                                    }
                                }
                                Ok(None) => {
                                    *err_msg =
                                        Some("Internal error in sending request.".to_owned());
                                }
                                Err(TryRecvError::Disconnected) => {
                                    if self.token.is_none() {
                                        *err_msg =
                                            Some("broken future/pipe encountered".to_owned());
                                    }
                                }
                                Err(TryRecvError::Empty) => (), // Do nothing
                            }
                        }

                        // if future was a failure, don't poll again
                        if err_msg.is_some() {
                            *signup_resp = None;
                        }

                        ui.label({
                            if let Some(ref msg) = *err_msg {
                                msg
                            } else {
                                ""
                            }
                        });
                    })
                    .response
                });
            });
        } else {
            unreachable!()
        }
        if successful {
            self.page = Page::Login {
                user: "".to_owned(),
                pass: "".to_owned(),
                login_resp: None,
                err_msg: None,
            };
        }
    }
}
