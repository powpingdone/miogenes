use crate::{error::*, MioClientState};
use anyhow::anyhow;
use base64::prelude::*;
use mio_common::*;

impl MioClientState {
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

    // this function merely refreshes the api token to call the server
    pub fn refresh_token(&mut self) -> Result<(), ErrorSplit> {
        let new_jwt = self
            .wrap_auth(self.agent.patch(&format!("{}/user/refresh", self.url)))
            .call()?
            .into_json::<auth::JWT>()?;
        if let Some(key) = self.key.get_mut() {
            *key = new_jwt;
        }
        Ok(())
    }

    // try login
    pub fn attempt_login(&mut self, username: &str, password: &str) -> Result<(), ErrorSplit> {
        let jwt = self
            .agent
            .get(&format!("{}/user/login", self.url))
            .set(
                "Authorization",
                &format!(
                    "Basic {}",
                    BASE64_URL_SAFE_NO_PAD.encode(format!("{username}:{password}"))
                ),
            )
            .call()?
            .into_json::<auth::JWT>()?;
        if let Some(key) = self.key.get_mut() {
            *key = jwt;
        }
        Ok(())
    }

    // try signup
    pub fn attempt_signup(&self, username: &str, password: &str) -> Result<(), ErrorSplit> {
        self.agent
            .post(&format!("{}/user/signup", self.url))
            .set(
                "Authorization",
                &format!(
                    "Basic {}",
                    BASE64_URL_SAFE_NO_PAD.encode(format!("{username}:{password}"))
                ),
            )
            .call()
            .map_err(ErrorSplit::from)?;
        Ok(())
    }
}
