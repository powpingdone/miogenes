use slint::{Model, SharedString, VecModel};

use crate::*;

impl MioFrontendWeak {
    pub fn setup_folders(&self) {}

    pub fn chdir(&self, at: String) {
        self.w_app()
            .unwrap()
            .global::<FolderSelectCB>()
            .get_at()
            .as_any()
            .downcast_ref::<VecModel<SharedString>>()
            .unwrap()
            .push(at.into());
        self.w_rt().unwrap().spawn(self.to_owned().regenerate());
    }

    pub fn up(&self) {
        let app = self.w_app().unwrap();
        let at_hold = app.global::<FolderSelectCB>();
        let dc_ref = at_hold.get_at();
        let b_ee = dc_ref.as_any().downcast_ref::<VecModel<SharedString>>();
        b_ee.unwrap().remove(b_ee.unwrap().row_count() - 1);
        self.w_rt().unwrap().spawn(self.to_owned().regenerate());
    }

    pub fn new_folder(&self) {}

    pub fn cancel(&self) {}

    pub fn upload(&self, at: Vec<String>) {}

    async fn regenerate(self) {
        self.app
            .upgrade_in_event_loop(|app| {
                app.global::<FolderSelectCB>().set_loaded(false);
            })
            .unwrap();
        let path = self
            .w_app()
            .unwrap()
            .global::<FolderSelectCB>()
            .get_at()
            .as_any()
            .downcast_ref::<VecModel<SharedString>>()
            .unwrap()
            .iter()
            .map(|x| x.as_str().to_owned())
            .collect::<Vec<String>>();
        // TODO: query directory contents
        todo!();
    }
}
