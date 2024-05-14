use std::path::PathBuf;

use crate::MioFrontendWeak;

impl MioFrontendWeak {
    pub fn send_upload(&self, path: PathBuf, at: String) -> tokio::task::JoinHandle<()> {
        self.w_rt()
            .unwrap()
            .spawn(self.clone().upload_bg_task(path, at))
    }

    async fn upload_bg_task(self, path: PathBuf, at: String) {
        // TODO: EVEN MORE ERROR HANDLING
        let h_state = self.w_state().unwrap();
        let state = h_state.read().await;
        state
            .upload_file(
                path.clone(),
                at,
                path
                    .file_name()
                    // to owned string
                    .and_then(|x| x.to_str().map(|x| x.to_owned())),
            )
            .await
            .unwrap();
    }
}
