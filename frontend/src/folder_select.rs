use std::rc::Rc;

use slint::{Model, ModelRc, SharedString, VecModel};

use crate::*;

impl MioFrontendWeak {
    // fetch folders
    pub fn setup_folders(&self) {}

    // goto dir
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

    // go up a dir
    pub fn up(&self) {
        let app = self.w_app().unwrap();
        let global_hold = app.global::<FolderSelectCB>();
        let at_hold = global_hold.get_at();
        let at = at_hold.as_any().downcast_ref::<VecModel<SharedString>>();
        at.unwrap().remove(at.unwrap().row_count() - 1);
        self.w_rt().unwrap().spawn(self.to_owned().regenerate());
    }

    // create dialog for text input for folder name
    pub fn new_folder(&self) {
        todo!()
    }

    // create folder and update
    pub fn confirmed_create_folder(&self, name: String) {
        let at = self
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
        let rt = self.w_rt().unwrap();
        rt.spawn({
            let this = self.to_owned();
            async move {
                let s_state = this.w_state().unwrap();
                let state = s_state.read().await;
                // make folder
                state.make_dir(name, at).await.unwrap();
                this.to_owned().regenerate().await;
            }
        });
    }

    // cancel upload
    pub fn cancel(&self) {
        self.unload_upload_page();
    }

    // begin upload
    pub fn upload(&self, at: Vec<String>) {
        // setup upload task
        todo!();
        self.unload_upload_page();
    }

    // change back to upload page
    fn unload_upload_page(&self) {
        self.app
            .upgrade_in_event_loop(|app| {
                // goto upload page
                app.global::<TopLevelCB>().set_page_t(TopLevelPage::Ready);
                app.global::<MainUICB>().set_page(MainUIPage::Upload);
                // and reset upload
                app.global::<FolderSelectCB>().set_loaded(false);
                app.global::<FolderSelectCB>().set_at(ModelRc::default());
                app.global::<FolderSelectCB>().set_cwd(ModelRc::default());
            })
            .unwrap();
    }

    // fetch dir contents
    async fn regenerate(self) {
        // unload
        self.app
            .upgrade_in_event_loop(|app| {
                app.global::<FolderSelectCB>().set_loaded(false);
            })
            .unwrap();
        // path to query
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
        // query directory contents
        let h_state = self.w_state().unwrap();
        let state = h_state.read().await;
        let listing = state.get_folder_listing(path).await.unwrap();
        // now update folders

        todo!();
    }
}