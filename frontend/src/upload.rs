use crate::MioFrontendWeak;
use native_dialog::FileDialog;
use std::{collections::HashSet, path::PathBuf};

impl MioFrontendWeak {
    pub fn send_upload(&self, path: PathBuf, at: String) -> tokio::task::JoinHandle<()> {
        let this = self.clone();
        self.w_rt().unwrap().spawn(async move {
            if path.is_dir() {
                this.upload_dir(path, at).await
            } else {
                this.upload_bg_task(path, at).await
            }
        })
    }

    pub fn file_dialog(&self) {
        let files = FileDialog::new()
            .set_title("Upload to Miogenes server")
            .show_open_multiple_file()
            .unwrap();
    }

    async fn upload_dir(self, path: PathBuf, at: String) {
        let h_state = self.w_state().unwrap();
        let state = h_state.read().await;
        let files = state.search_folder(&path).await.unwrap();

        // get folders needed to be created
        let new_folders = tokio::task::block_in_place(|| {
            let mut new_folders = HashSet::new();
            for found in files.iter() {
                let mut recon_path = vec![path.parent().unwrap().as_os_str()];
                for pt in found.strip_prefix(&path).unwrap().components() {
                    if let std::path::Component::Normal(cmp) = pt {
                        recon_path.push(cmp);
                    } else if let std::path::Component::ParentDir = pt {
                        recon_path.pop();
                    }
                }
                new_folders.insert(
                    recon_path
                        .into_iter()
                        .map(|x| x.to_owned())
                        .collect::<Vec<_>>(),
                );
            }
            new_folders
        });

        // create folders
        let mut folder_tasks = Vec::with_capacity(new_folders.len());
        for folder in new_folders.into_iter() {
            folder_tasks.push(tokio::spawn({
                let mut pstack = vec![at.clone()];
                let h_state = h_state.clone();
                async move {
                    let state = h_state.read().await;
                    for pt in folder {
                        let pt = pt.to_string_lossy().into_owned();

                        // TODO: possibly not make this copy every single time
                        state.make_dir(pt.clone(), pstack.clone()).await.unwrap();
                        pstack.push(pt);
                    }
                }
            }));
        }
        for task in folder_tasks.into_iter() {
            drop(task.await);
        }

        // finally, send off the upload tasks
        for file in files.into_iter() {
            let mut t_at = at.clone();
            t_at.push_str(
                &file
                    .parent()
                    .unwrap()
                    .strip_prefix(&path)
                    .unwrap()
                    .to_string_lossy(),
            );
            let this = self.clone();
            tokio::spawn(this.upload_bg_task(file, t_at));
        }
    }

    async fn upload_bg_task(self, path: PathBuf, at: String) {
        // TODO: EVEN MORE ERROR HANDLING
        //
        // TODO: update ui with progress
        let h_state = self.w_state().unwrap();
        let state = h_state.read().await;
        state
            .upload_file(
                path.clone(),
                at,
                path.file_name()
                    // to owned string
                    .and_then(|x| Some(x.to_string_lossy().into_owned())),
            )
            .await
            .unwrap();

        // wake up albums tab
        crate::albums::WAKE_UP.get().unwrap().send(()).unwrap();
    }
}
