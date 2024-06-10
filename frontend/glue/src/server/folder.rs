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

    pub async fn get_folders(&self) -> GlueResult<HashMap<String, Interm>> {
        // fetch from server
        let raw_tree = self
            .wrap_auth(self.agent.get(&format!("{}/api/folder", self.url)))
            .send()
            .await?
            .json::<retstructs::FolderQuery>()
            .await?
            .ret;

        // turn into hashmap tree
        //
        // TODO: does this need to be Option?
        Ok(tokio::task::spawn_blocking(move || {
            let split_tree: Vec<Vec<_>> = raw_tree
                .iter()
                .map(|x| x.split('/').collect::<Vec<_>>())
                .collect();
            let mut hmap_master = Some(HashMap::<String, Interm>::new());
            for branch in split_tree {
                let mut curr_hmap: &mut Option<HashMap<String, Interm>> = &mut hmap_master;
                for leaf in branch {
                    // check if end of branch exists yet
                    if curr_hmap.is_none() {
                        *curr_hmap = Some(HashMap::new());
                    }

                    // check if branch contains the folder
                    let mutref = curr_hmap.as_mut().unwrap();
                    if !mutref.contains_key(leaf) {
                        mutref.insert(leaf.to_owned(), Interm(None));
                    }

                    // finally, get next branch
                    curr_hmap = mutref.get_mut(leaf).map(|x| &mut x.0).unwrap();
                }
            }
            hmap_master.unwrap()
        })
        .await
        .map_err(|err| anyhow::Error::from(err))?)
    }
}
