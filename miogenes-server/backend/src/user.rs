use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::{FromRequest, Query, RequestParts};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Extension, Json};
use log::*;
use mio_common::msgstructs::UserToken;
use serde::*;
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use uuid::Uuid;

use crate::{db_deser, MioError, MioState};

#[derive(Deserialize, Debug, Clone)]
pub struct DBUserToken {
    pub id: String,
    pub token: Uuid,
    pub is_expired: bool,
}

#[serde_as]
#[derive(Deserialize, Debug, Clone)]
pub struct User {
    // internal structs
    pub id: Option<String>,
    pub tokens: Option<Vec<DBUserToken>>,

    // external structs
    #[serde(alias = "u")]
    pub username: String,
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(alias = "h")]
    pub password: [u8; 32],
}

static TIMEOUT_TIME: i64 = 3;

pub(crate) struct Authenticate;

#[async_trait]
impl<B> FromRequest<B> for Authenticate
where
    B: Send,
{
    type Rejection = (StatusCode, Json<MioError>);
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(state) = req.extract::<Extension<Arc<MioState>>>().await.unwrap();
        let Query(UserToken(token)) = req.extract::<Query<UserToken>>().await.unwrap();
        let user: User = match state
            .db
            .execute(
                "SELECT * FROM user WHERE tokens[WHERE id = $token]",
                &state.sess,
                Some([("token".to_owned(), token.to_string().into())].into()),
                false,
            )
            .await
        {
            Ok(mut query) => {
                let deser = {
                    match query.pop().ok_or(anyhow!("db returned no user")) {
                        Ok(query) => db_deser(query),
                        Err(err) => Err(err),
                    }
                };

                match deser {
                    Ok(user) => Ok(user),
                    Err(err) => {
                        debug!("error authenticating user: {err}");
                        Err((
                            StatusCode::NOT_ACCEPTABLE,
                            Json(MioError {
                                msg: "invalid token".to_owned(),
                            }),
                        ))
                    }
                }
            }
            Err(err) => {
                error!("db request error: {err}");
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(MioError {
                        msg: "internal db error".to_owned(),
                    }),
                ))
            }
        }?;
        if let Some(item) = req.extensions_mut().insert(user) {
            warn!(
                "warning while injecting user: user of {:?} already existed. replacing.",
                item.username
            );
        }
        Ok(Authenticate)
    }
}

pub async fn login(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(user): Query<User>,
) -> impl IntoResponse {
    let user: User = {
        match state
            .db
            .execute(
                "SELECT * FROM user WHERE password = $password AND username = $username;",
                &state.sess,
                Some(
                    [
                        ("username".to_owned(), user.username.into()),
                        ("password".to_owned(), user.password.to_vec().into()),
                    ]
                    .into(),
                ),
                false,
            )
            .await
        {
            Ok(mut query) => {
                let deser = {
                    match query.pop().ok_or(anyhow!("db returned no user")) {
                        Ok(query) => db_deser(query),
                        Err(err) => Err(err),
                    }
                };

                match deser {
                    Ok(user) => user,
                    Err(err) => {
                        debug!("error authenticating user: {err}");
                        return Err((
                            StatusCode::NOT_ACCEPTABLE,
                            Json(MioError {
                                msg: "invalid username or password".to_owned(),
                            }),
                        ));
                    }
                }
            }
            Err(err) => {
                error!("internal db error: {err}");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(MioError {
                        msg: "internal db error".to_owned(),
                    }),
                ));
            }
        }
    };

    let new_token = Uuid::new_v4();
    // TODO: let server host specify when logout tokens expire
    let expiry = chrono::Utc::now() + chrono::Duration::days(TIMEOUT_TIME);
    state
        .db
        .execute(
            &format!("CREATE user_token:`{new_token}` SET expires = $expires;"),
            &state.sess,
            Some([("expires".to_owned(), expiry.into())].into()),
            false,
        )
        .await
        .unwrap()
        .pop()
        .unwrap();
    state
        .db
        .execute(
            &format!(
                "UPDATE {} SET tokens += [user_token:`{new_token}`];",
                user.id.unwrap()
            ),
            &state.sess,
            None,
            false,
        )
        .await
        .unwrap()
        .pop()
        .unwrap();

    Ok((StatusCode::OK, Json(UserToken(new_token))))
}

pub async fn refresh_token(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(UserToken(token)): Query<UserToken>,
) -> impl IntoResponse {
    let ret = state
        .db
        .execute(
            &format!(
                "UPDATE user_token:`{token}` SET expires = $expires WHERE is_expired = false;"
            ),
            &state.sess,
            Some(
                [(
                    "expires".to_owned(),
                    (chrono::Utc::now() + chrono::Duration::days(TIMEOUT_TIME)).into(),
                )]
                .into(),
            ),
            false,
        )
        .await;
    if ret.is_err() || ret.unwrap().pop().is_none() {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::OK
    }
}

pub async fn logout(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(UserToken(token)): Query<UserToken>,
) -> impl IntoResponse {
    todo!()
}
