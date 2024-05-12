use crate::{error::GlueResult, MioClientState};
use mio_common::*;
use std::collections::HashMap;

pub struct FakeMapItem {
    pub key: String,
    pub value: Option<Vec<FakeMapItem>>,
}

impl MioClientState {
    pub fn make_dir(&self, name: String, path: String) -> GlueResult<()> {
        self.wrap_auth(self.agent.put(&format!(
            "{}/api/folder?{}",
            self.url,
            serde_urlencoded::to_string(msgstructs::FolderCreateDelete { name, path }).unwrap()
        )))
        .call()?;
        Ok(())
    }

    pub fn get_folders(&self) -> GlueResult<Vec<FakeMapItem>> {
        // fetch from server
        let raw_tree = self
            .wrap_auth(self.agent.get(&format!("{}/api/folder", self.url)))
            .call()?
            .into_json::<retstructs::FolderQuery>()?
            .ret;
        let split_tree: Vec<Vec<_>> = raw_tree
            .iter()
            .map(|x| x.split('/').collect::<Vec<_>>())
            .collect();

        // turn into hashmap tree
        struct Interm(Option<HashMap<String, Interm>>);

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

        // then, turn into fakemap
        fn create_fakemap(map: Option<HashMap<String, Interm>>) -> Option<Vec<FakeMapItem>> {
            if let Some(map) = map {
                let mut ret = Vec::with_capacity(map.len());
                for (key, value) in map {
                    ret.push(FakeMapItem {
                        key,
                        value: create_fakemap(value.0),
                    });
                }
                Some(ret)
            } else {
                None
            }
        }

        Ok(create_fakemap(hmap_master).unwrap())
    }
}
