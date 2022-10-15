use axum::{Extension, async_trait, Json};
use axum::extract::{Query, FromRequest, RequestParts};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use serde::*;
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use uuid::Uuid;

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct User {
    #[serde(alias = "i")]
    pub userid: Option<Uuid>,
    #[serde(alias = "u")]
    pub username: String,
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(alias = "h")]
    pub password: [u8; 32],
}

pub(crate) struct Authenticate;

#[async_trait]
impl<B> FromRequest<B> for Authenticate where B: Send {
    type Rejection = (StatusCode, Json<crate::MioError>);
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        todo!()
    }
}

pub async fn login(
    Extension(state): Extension<crate::MioState>,
    Query(user): Query<User>,
) -> impl IntoResponse {
    todo!()
}

pub async fn refresh_token(
    Extension(state): Extension<crate::MioState>,
    Query(token): Query<mio_common::msgstructs::UserToken>
) -> impl IntoResponse {
    todo!()
}

pub async fn logout(
    Extension(state): Extension<crate::MioState>,
    Query(token): Query<mio_common::msgstructs::UserToken>,
) -> impl IntoResponse {
    todo!()
}