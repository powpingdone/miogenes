use crate::{
    error::{ErrorSplit, GlueResult},
    MioClientState,
};
use anyhow::anyhow;
use mio_common::*;
use std::{
    fs::{read, read_dir},
    path::Path,
    pin::Pin, future::Future,
};

impl MioClientState {
    // recursive function for searching for audio files
    pub async fn search_folder(&self, path: impl AsRef<Path>) -> GlueResult<Vec<String>> {
         search_folder_inner(path).await
        
    }

    pub async fn upload_file(
        &self,
        fullpath: impl AsRef<Path>,
        dir: String,
        fname: Option<String>,
    ) -> Result<retstructs::UploadReturn, ErrorSplit> {
        // TODO: this is also not async
        let buf = read(fullpath).map_err(|err| anyhow!("Failed to read file: {err}"))?;
        Ok(self
            .wrap_auth(self.agent.post(&format!("{}/api/track?", self.url,)))
            .query(&msgstructs::TrackUploadQuery { dir, fname })
            .body(buf)
            .send()
            .await?
            .json::<retstructs::UploadReturn>()
            .await?)
    }
}

fn search_folder_inner(path: impl AsRef<Path>) -> Pin<Box<dyn Future<Output = GlueResult<Vec<String>>>>> {
    let path = path.as_ref().to_owned();

    Box::pin(async move {
        // TODO: add opus support
        //
        // TODO: this is not async
        const COMMON_EXTS: &[&str] = &["wav", "flac", "alac", "mp3", "ogg", "aac", "m4a"];
        let mut ret = vec![];
        let dir = read_dir(&path)?;
        for item in dir {
            let item = item?;
            let ftype = item.file_type()?;
            let mut path_copy = path.to_owned();
            if ftype.is_file() {
                // check if file has common audio extension
                //
                // TODO: MAYBE use libmagic/gst to check if file is actually audio
                path_copy.push(item.file_name());
                if COMMON_EXTS.iter().any(|ext| {
                    path_copy
                        .extension()
                        .map(|x| ext == &x.to_string_lossy())
                        .unwrap_or_default()
                }) {
                    ret.push(
                        path_copy
                            .to_str()
                            .ok_or_else(|| {
                                anyhow!("Failed to read {} as UTF-8", path_copy.display())
                            })?
                            .to_owned(),
                    );
                }
            } else if ftype.is_dir() {
                // recurse here
                path_copy.push(item.file_name());
                let path_clone = path_copy.clone();
                ret.extend(search_folder_inner(path_clone).await?);
            }
        }
        Ok(ret)
    })
}
