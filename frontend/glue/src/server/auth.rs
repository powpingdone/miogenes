use crate::{error::*, MioClientState};
use anyhow::anyhow;
use mio_common::*;

impl MioClientState {
    pub async fn test_set_url(&mut self, url: String) -> GlueResult<()> {
        use konst::primitive::parse_u16;
        use konst::unwrap_ctx;

        let vers: Vers = self
            .agent
            .get_ref()
            .get(&format!("{url}/ver"))
            .send()
            .await
            .map_err(|err| {
                anyhow!("This miogenes server doesn't seem to exist: tried contacting, got {err}")
            })?
            .json()
            .await
            .map_err(|err| anyhow!("This is not a miogenes server. Serialization error: {err}"))?;
        if vers
            != Vers::new(
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
                unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
            )
        {
            return Err(anyhow!("Version mismatch! Update the mobile app or the server.").into());
        }
        self.url = url;
        Ok(())
    }

    // this function merely refreshes the api token to call the server
    pub async fn refresh_token(&mut self) -> GlueResult<()> {
        let new_jwt = self
            .wrap_auth(
                self.agent
                    .get_ref()
                    .patch(&format!("{}/user/refresh", self.url)),
            )
            .send()
            .await?
            .json::<auth::JWT>()
            .await?;
        if let Some(key) = self.key.get_mut() {
            *key = new_jwt;
        } else {
            self.key.set(new_jwt).unwrap();
        }
        Ok(())
    }

    // try login
    pub async fn attempt_login(&mut self, username: &str, password: &str) -> GlueResult<()> {
        let jwt = self
            .agent
            .get_ref()
            .get(&format!("{}/user/login", self.url))
            .basic_auth(username, Some(password))
            .send()
            .await?
            .json::<auth::JWT>()
            .await?;

        if let Some(key) = self.key.get_mut() {
            *key = jwt;
        } else {
            self.key.set(jwt).unwrap();
        }
        Ok(())
    }

    // try signup
    pub async fn attempt_signup(&self, username: &str, password: &str) -> GlueResult<()> {
        self.agent
            .get_ref()
            .post(&format!("{}/user/signup", self.url))
            .basic_auth(username, Some(password))
            .send()
            .await?;
        Ok(())
    }
}
