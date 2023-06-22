mod bridge_generated;

use std::{fmt::Display, ops::DerefMut, sync::RwLock};

use anyhow::anyhow;
use mio_common::*;
use ureq::Agent;
mod api;

pub enum ErrorSplit<T: Display> {
    Ureq(ureq::Error),
    Other(T),
}

#[derive(Debug)]
pub struct MioClientState {
    url: String,
    agent: Agent,
    pub key: RwLock<Option<auth::JWT>>,
}

impl MioClientState {
    pub fn new() -> Self {
        Self {
            url: "".to_owned(),
            agent: ureq::agent(),
            key: RwLock::new(None),
        }
    }

    pub fn test_set_url(&mut self, url: String) -> anyhow::Result<()> {
        use konst::primitive::parse_u16;
        use konst::unwrap_ctx;

        let vers: Vers = self
            .agent
            .get(&format!("{url}/ver"))
            .call()
            .map_err(|err| {
                anyhow!("This miogenes server doesn't seem to exist: tried contacting, got {err}")
            })?
            .into_json()
            .map_err(|err| anyhow!("This is not a miogenes server. Serialization error: {err}"))?;
        if vers
            != Vers::new(
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
            )
        {
            anyhow::bail!("Version mismatch! Update the mobile app or the server.")
        }
        self.url = url;
        Ok(())
    }

    pub fn refresh_token(&mut self) -> Result<(), ureq::Error> {
        let new_jwt = self
            .wrap_auth(self.agent.patch(&format!("{}/user/refresh", self.url)))
            .call()?
            .into_json::<auth::JWT>()?;
        self.key.write().unwrap().deref_mut().replace(new_jwt);
        Ok(())
    }

    fn wrap_auth(&self, req: ureq::Request) -> ureq::Request {
        use base64::prelude::*;
        req.set(
            "Authorization",
            &format!(
                "Bearer {}",
                BASE64_URL_SAFE_NO_PAD
                    .encode(self.key.read().unwrap().as_ref().unwrap().to_string())
            ),
        )
    }
}
