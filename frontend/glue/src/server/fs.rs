use crate::{error::ErrorSplit, MioClientState};
use anyhow::anyhow;
use mio_common::*;
use std::{
    fs::{read, read_dir},
    path::Path,
};

impl MioClientState {
    // recursive function for searching for audio files
    pub fn search_folder(&self, path: impl AsRef<Path>) -> Result<Vec<String>, ErrorSplit> {
        self.search_folder_inner(path)
    }

    fn search_folder_inner(&self, path: impl AsRef<Path>) -> Result<Vec<String>, ErrorSplit> {
        let path = path.as_ref();
        const COMMON_EXTS: &[&str] = &["wav", "flac", "alac", "mp3", "ogg", "aac", "opus", "m4a"];
        let mut ret = vec![];
        let dir = read_dir(path)?;
        for item in dir {
            let item = item?;
            let ftype = item.file_type()?;
            if ftype.is_file() {
                // check if file has common audio extension
                //
                // TODO: MAYBE use libmagic/gst to check if file is actually audio
                let mut new_path = path.to_owned();
                new_path.push(item.file_name());
                if COMMON_EXTS.iter().any(|ext| {
                    new_path
                        .extension()
                        .map(|x| ext == &x.to_string_lossy())
                        .unwrap_or_default()
                }) {
                    ret.push(
                        new_path
                            .to_str()
                            .ok_or_else(|| {
                                anyhow!("Failed to read {} as UTF-8", new_path.display())
                            })?
                            .to_owned(),
                    );
                }
            } else if ftype.is_dir() {
                let mut travel = path.to_owned();
                travel.push(item.file_name());
                ret.extend(self.search_folder(travel)?);
            }
        }
        Ok(ret)
    }

    pub fn upload_file(
        &self,
        fullpath: impl AsRef<Path>,
        dir: String,
        fname: Option<String>,
    ) -> Result<retstructs::UploadReturn, ErrorSplit> {
        let buf = read(fullpath).map_err(|err| anyhow!("Failed to read file: {err}"))?;
        Ok(self
            .wrap_auth(self.agent.post(&format!(
                "{}/api/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::TrackUploadQuery { dir, fname }).unwrap()
            )))
            .send_bytes(&buf)?
            .into_json::<retstructs::UploadReturn>()?)
    }
}
