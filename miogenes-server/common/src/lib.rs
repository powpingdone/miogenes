use serde::*;

pub mod retstructs;
pub mod msgstructs;

#[derive(Serialize, Deserialize)]
pub struct Vers {
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
        }
    }
}
