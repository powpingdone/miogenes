use serde::*;
use uuid::Uuid;

pub mod auth;
pub mod msgstructs;
pub mod retstructs;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Vers {
    special_key0: Uuid,
    special_key1: Uuid,
    major: u16,
    minor: u16,
    patch: u16,
}

impl Vers {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Vers {
        Vers {
            major,
            minor,
            patch,
            special_key0: uuid::uuid!("ddf6b403-6a16-4b65-92e0-8342cad3c3e1"),
            special_key1: uuid::uuid!("b39120cb-f4be-49b5-93ef-9da95610df7d"),
        }
    }
}
