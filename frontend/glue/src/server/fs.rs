use crate::{
    error::{GlueError, GlueResult},
    MioClientState,
};
use anyhow::anyhow;
use futures::StreamExt;
use mio_common::*;
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::fs::{read, read_dir};

impl MioClientState {
    // recursive function for searching for audio files
    pub async fn search_folder(&self, path: impl AsRef<Path>) -> GlueResult<Vec<PathBuf>> {
        search_folder_inner(path).await
    }

    pub async fn upload_file(
        &self,
        fullpath: impl AsRef<Path>,
        dir: String,
        fname: Option<String>,
    ) -> Result<retstructs::UploadReturn, GlueError> {
        let buf = read(fullpath)
            .await
            .map_err(|err| anyhow!("Failed to read file: {err}"))?;
        Ok(self
            .wrap_auth(self.agent.post(&format!("{}/api/track?", self.url)))
            .query(&msgstructs::TrackUploadQuery { dir, fname })
            .body(buf)
            .send()
            .await?
            .json::<retstructs::UploadReturn>()
            .await?)
    }
}

fn search_folder_inner(
    path: impl AsRef<Path>,
) -> Pin<Box<dyn Send + Future<Output = GlueResult<Vec<PathBuf>>>>> {
    let path = path.as_ref().to_owned();
    Box::pin(async move {
        // TODO: add opus support
        const COMMON_EXTS: &[&str] = &["wav", "flac", "alac", "mp3", "ogg", "aac", "m4a"];
        let mut ret = vec![];
        let mut dir = tokio_stream::wrappers::ReadDirStream::new(read_dir(&path).await?);
        while let Some(item) = dir.next().await {
            let item = item?;
            let ftype = item.file_type().await?;
            let mut path_copy = path.to_owned();
            path_copy.push(item.file_name());
            if ftype.is_file() {
                // check if file has common audio extension
                //
                // TODO: MAYBE use libmagic/gst to check if file is actually audio
                if COMMON_EXTS.iter().any(|ext| {
                    path_copy
                        .extension()
                        .map(|x| ext == &x.to_string_lossy())
                        .unwrap_or_default()
                }) {
                    ret.push(path_copy);
                }
            } else if ftype.is_dir() {
                // recurse here
                ret.extend(search_folder_inner(path_copy).await?);
            }
        }
        Ok(ret)
    })
}
