use base64::prelude::*;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct JWT(String);

impl std::fmt::Debug for JWT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("JWT").field(&"**SCRUBBED**").finish()
    }
}

impl ToString for JWT {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

const ALG: Algorithm = Algorithm::HS512;

impl JWT {
    pub fn new(inner: JWTInner, secret: &[u8]) -> jsonwebtoken::errors::Result<Self> {
        Ok(JWT(jsonwebtoken::encode(
            &Header {
                alg: ALG,
                ..Default::default()
            },
            &inner,
            &EncodingKey::from_secret(secret),
        )?))
    }

    pub fn from_raw(x: String) -> Self {
        Self(x)
    }

    pub fn whois(&self) -> Result<JWTInner, anyhow::Error> {
        Ok(serde_json::from_slice(
            &BASE64_URL_SAFE_NO_PAD.decode(
                self
                    .0
                    .split('.')
                    .nth(1)
                    .ok_or_else(|| anyhow::anyhow!("no payload to decode"))?,
            )?,
        )?)
    }

    pub fn decode(self, secret: &[u8]) -> jsonwebtoken::errors::Result<TokenData<JWTInner>> {
        jsonwebtoken::decode(
            &self.0,
            &DecodingKey::from_secret(secret),
            &Validation::new(ALG),
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct JWTInner {
    pub userid: Uuid,
    pub exp: i64,
}
