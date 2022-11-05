use std::sync::Arc;

use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::extract::{FromRequest, Query, RequestParts};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Extension, Json, TypedHeader};
use chrono::Duration;
use log::*;
use mio_common::msgstructs::UserToken;
use serde::*;
use sled::transaction::abort;
use uuid::Uuid;

use crate::db::{Index, Table};
use crate::{MioError, MioState};

#[derive(Deserialize, Debug, Clone)]
pub struct DBUserToken {
    pub id: String,
    pub token: Uuid,
    pub is_expired: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    // internal structs
    pub id: Option<String>,
    pub tokens: Option<Vec<DBUserToken>>,

    // external structs
    pub username: String,
    pub password: String,
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
        let auth = req
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .unwrap();
        let user: User = match state
            .db
            .execute(
                "SELECT * FROM user WHERE tokens[WHERE id = $token];",
                &state.sess,
                Some([("token".to_owned(), auth.token().into())].into()),
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
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> impl IntoResponse {
    let user: User = {
        match state
            .db
            .execute(
                "SELECT * FROM user WHERE username = $username LIMIT 1;",
                &state.sess,
                Some([("username".to_owned(), auth.username().into())].into()),
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

    // check hash
    tokio::task::spawn_blocking({
        move || {
            let parsed = PasswordHash::new(&user.password).map_err(|err| {
                error!("unable to extract phc string: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(MioError {
                        msg: "internal server error".to_owned(),
                    }),
                )
            })?;
            Argon2::default()
                .verify_password(auth.password().to_owned().as_bytes(), &parsed)
                .map_err(|err| {
                    debug!("unable to verify password: {err}");
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(MioError {
                            msg: "invalid username or password".to_owned(),
                        }),
                    )
                })
        }
    })
    .await
    .map_err(|err| {
        error!("task failed to start: {err}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(MioError {
                msg: "internal server error".to_owned(),
            }),
        )
    })??;

    // gen new token
    let new_token = Uuid::new_v4();
    // TODO: let server host specify when logout tokens expire
    let expiry = chrono::Utc::now() + chrono::Duration::days(TIMEOUT_TIME);
    state
        .db
        .execute(
            &format!(
                "BEGIN;
                CREATE user_token:`{new_token}` SET expires = $expires;
                UPDATE {} SET tokens += [user_token:`{new_token}`];
                COMMIT;",
                user.id.unwrap()
            ),
            &state.sess,
            Some([("expires".to_owned(), expiry.into())].into()),
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
        .open_tree(Table::UserToken)
        .unwrap()
        .transaction(|tx_db| {
            let timeup: Result<Index<crate::db::UserToken>, _> = Index::new(token, &{
                let x = tx_db.get(token)?;
                if x.is_none() {
                    abort(anyhow!("no user found"))?;
                }
                x.unwrap()
            }); 
            if timeup.is_err() {
                abort(anyhow!("serialization err"))?;
            }
            let timeup = timeup.unwrap();
            
            if !timeup.inner().is_expired() {
                timeup
                    .inner_mut()
                    .push_forward(Duration::days(TIMEOUT_TIME));
            }
            
            tx_db.insert(token.as_bytes(), timeup.consume().unwrap())?;

            Ok(())
        });
    if ret.is_err() {
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
