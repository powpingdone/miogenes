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
use sled::transaction::abort;
use uuid::Uuid;

use crate::db::{self, Index, TopLevel, User, UserToken};
use crate::{MioError, MioState};
use mio_common::*;

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
        let auth = Uuid::parse_str(
            req.extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .unwrap()
                .token(),
        )
        .map_err(|err| {
            debug!("could not parse token: {err}");
            (
                StatusCode::BAD_REQUEST,
                Json(MioError {
                    msg: "invalid user token".to_owned(),
                }),
            )
        })?;
        let user: Index<UserToken> = {
            let mut user: Option<Index<UserToken>> = None;
            for x in state.db.open_tree(TopLevel::UserToken).unwrap().iter() {
                match x {
                    Err(err) => {
                        error!("internal database error: {err}");
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(MioError {
                                msg: "internal server error".to_owned(),
                            }),
                        ));
                    }
                    Ok((key, value)) => {
                        let key: Uuid = Uuid::from_slice(&key).map_err(|err| {
                            error!("failed to serialize uuid: {err}");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(MioError {
                                    msg: "internal server error".to_owned(),
                                }),
                            )
                        })?;
                        let index = Index::<UserToken>::new(key, &value).map_err(|err| {
                            error!("failed to serialize user struct: {err}");
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(MioError {
                                    msg: "internal server error".to_owned(),
                                }),
                            )
                        })?;
                        if index.id() == auth {
                            user = Some(index)
                        }
                    }
                }
            }
            if let Some(ret) = user {
                ret
            } else {
                debug!("token not found: {}", auth);
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(MioError {
                        msg: "invalid token".to_owned(),
                    }),
                ));
            }
        };

        let user = Index::<User>::new(
            user.inner().user(),
            &state
                .db
                .open_tree(TopLevel::User)
                .unwrap()
                .get(user.inner().user())
                .unwrap()
                .ok_or_else(|| {
                    error!("could not find user with token {}", user.inner().user());
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(MioError {
                            msg: "internal server error".to_owned(),
                        }),
                    )
                })?,
        )
        .map_err(|err| {
            error!("failed to serialize user: {err}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(MioError {
                    msg: "internal server error".to_owned(),
                }),
            )
        })?;

        if let Some(item) = req.extensions_mut().insert(user) {
            warn!(
                "warning while injecting user: user of {:?} already existed. replacing.",
                item.inner().username()
            );
        }

        Ok(Authenticate)
    }
}

pub async fn login(
    Extension(state): Extension<Arc<crate::MioState>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> impl IntoResponse {
    let user: Index<User> = {
        let mut user: Option<Index<User>> = None;
        for x in state.db.open_tree(TopLevel::User).unwrap().iter() {
            match x {
                Err(err) => {
                    error!("internal database error: {err}");
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(MioError {
                            msg: "internal server error".to_owned(),
                        }),
                    ));
                }
                Ok((key, value)) => {
                    let key: Uuid = Uuid::from_slice(&key).map_or_else(
                        |err| {
                            error!("failed to serialize uuid: {err}");
                            Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(MioError {
                                    msg: "internal server error".to_owned(),
                                }),
                            ))
                        },
                        |val| Ok(val),
                    )?;
                    let index = Index::<User>::new(key, &value).map_or_else(
                        |err| {
                            error!("failed to serialize user struct: {err}");
                            Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(MioError {
                                    msg: "internal server error".to_owned(),
                                }),
                            ))
                        },
                        |val| Ok(val),
                    )?;
                    if index.inner().username() == auth.username() {
                        user = Some(index);
                        break;
                    }
                }
            }
        }
        if let Some(ret) = user {
            ret
        } else {
            debug!("user not found: {}", auth.username());
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(MioError {
                    msg: "invalid username or password".to_owned(),
                }),
            ));
        }
    };

    // check hash
    let passwd = user.inner().password().to_owned();
    tokio::task::spawn_blocking({
        move || {
            let parsed = PasswordHash::new(&passwd).map_err(|err| {
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
        .open_tree(TopLevel::UserToken)
        .unwrap()
        .insert(new_token, UserToken::generate(user.id(), expiry));

    Ok((StatusCode::OK, Json(msgstructs::UserToken(new_token))))
}

pub async fn refresh_token(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    let ret = state
        .db
        .open_tree(TopLevel::UserToken)
        .unwrap()
        .transaction(|tx_db| {
            let timeup: Result<Index<db::UserToken>, _> = Index::new(token, &{
                let x = tx_db.get(token)?;
                if x.is_none() {
                    abort(anyhow!("no user found"))?;
                }
                x.unwrap()
            });
            if timeup.is_err() {
                abort(anyhow!("serialization err"))?;
            }
            let mut timeup = timeup.unwrap();

            if !timeup.inner().is_expired() {
                timeup
                    .inner_mut()
                    .push_forward(Duration::days(TIMEOUT_TIME));
            }

            tx_db.insert(token.as_bytes(), timeup.decompose().unwrap())?;

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
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    todo!()
}
