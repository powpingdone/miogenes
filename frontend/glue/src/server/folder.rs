use crate::{error::GlueResult, MioClientState};
use mio_common::*;
use std::collections::HashMap;

pub struct Interm(Option<HashMap<String, Interm>>);

impl MioClientState {
    pub async fn make_dir(&self, name: String, path: Vec<String>) -> GlueResult<()> {
        self.wrap_auth(self.agent.put(&format!("{}/api/folder", self.url)))
            .query(&msgstructs::FolderCreateDelete { name, path })
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_folder_listing(
        &self,
        path: Vec<String>,
    ) -> GlueResult<Vec<retstructs::FolderQueryItem>> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/folder", self.url)))
            .query(&msgstructs::FolderQuery { path })
            .send()
            .await?
            .json::<retstructs::FolderQuery>()
            .await?
            .ret
            .tree
            .unwrap_or_default())
    }

    pub async fn get_folders(&self) -> GlueResult<retstructs::FolderQueryItem> {
        // fetch from server
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/folder", self.url)))
            .send()
            .await?
            .json::<retstructs::FolderQuery>()
            .await?
            .ret)
    }
}
