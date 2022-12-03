use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequestParts, Query, State};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Json, RequestPartsExt, TypedHeader};
use chrono::Utc;
use log::*;
use rand::rngs::OsRng;
use sea_orm::{prelude::*, *};
use uuid::Uuid;

use crate::{db_err, tr_conv_code, MioInnerError, MioState};
use mio_common::*;
use mio_entity::{user, user_token, User, UserToken};

static TIMEOUT_TIME_DAY: i64 = 3;

pub(crate) struct Authenticate;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticate
where
    S: Send + Sync + TransactionTrait,
{
    type Rejection = StatusCode;
    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // get user token
        let auth = Uuid::parse_str(
            req.extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .map_err(|err| -> StatusCode {
                    MioInnerError::UserChallengedFail(
                        Level::Debug,
                        anyhow!("auth header failure: {err}"),
                        StatusCode::BAD_REQUEST,
                    )
                    .into()
                })?
                .token(),
        )
        .map_err(|err| -> StatusCode {
            debug!("USER_INJ could not parse token: {err}");
            MioInnerError::UserChallengedFail(
                Level::Debug,
                anyhow!("could not parse token: {err}"),
                StatusCode::BAD_REQUEST,
            )
            .into()
        })?;
        let user = state
            .transaction(|txn| {
                Box::pin(async move {
                    // check for existence and validity of token
                    let usertoken =
                        UserToken::find_by_id(auth).one(txn).await?.ok_or_else(|| {
                            MioInnerError::NotFound(Level::Debug, anyhow!("user token: {auth}"))
                        })?;
                    if usertoken.expiry > Utc::now() {
                        return Err(MioInnerError::UserChallengedFail(
                            Level::Debug,
                            anyhow!(
                                "expired token used, rejecting: {} (expiry was {})",
                                usertoken.id,
                                usertoken.expiry
                            ),
                            StatusCode::UNAUTHORIZED,
                        ));
                    }

                    // fetch user
                    let user = User::find_by_id(usertoken.user_id)
                        .one(txn)
                        .await?
                        .ok_or_else(|| {
                            MioInnerError::UserChallengedFail(
                                Level::Error,
                                anyhow!("usertoken found, but no user found"),
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        })?;
                    Ok(user)
                })
            })
            .await
            .map_err(tr_conv_code)?;

        // inject user
        if let Some(item) = req.extensions.insert(user) {
            warn!(
                "USER_INJ while injecting user: user of {} existed, replacing.",
                item.username
            );
        }

        Ok(Authenticate)
    }
}

pub async fn login(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> impl IntoResponse {
    state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                // get user
                let user = User::find()
                    .filter(user::Column::Username.eq(auth.username()))
                    .one(txn)
                    .await?
                    .ok_or_else(|| {
                        MioInnerError::NotFound(
                            Level::Debug,
                            anyhow!("failed to find user \"{}\" in db", auth.username()),
                        )
                    })?;

                // check hash
                let passwd = user.password.to_owned();
                tokio::task::block_in_place({
                    move || {
                        let parsed = PasswordHash::new(&passwd).map_err(|err| {
                            MioInnerError::UserChallengedFail(
                                Level::Error,
                                anyhow!("unable to extract phc string: {err}"),
                                StatusCode::INTERNAL_SERVER_ERROR,
                            )
                        })?;
                        Argon2::default()
                            .verify_password(auth.password().to_owned().as_bytes(), &parsed)
                            .map_err(|err| {
                                MioInnerError::UserChallengedFail(
                                    Level::Debug,
                                    anyhow!("unable to verify password: {err}"),
                                    StatusCode::UNAUTHORIZED,
                                )
                            })
                    }
                })?;

                // generate new token
                let new_token = Uuid::new_v4();
                // TODO: let server host specify when logout tokens expire
                let expiry = Utc::now() + chrono::Duration::days(TIMEOUT_TIME_DAY);
                user_token::ActiveModel {
                    id: Set(new_token),
                    expiry: Set(expiry),
                    user_id: Set(user.id),
                }
                .insert(txn)
                .await?;

                debug!(
                    "GET /l/login new token generated for {}: {new_token}, expires {expiry}",
                    user.id
                );
                Ok((StatusCode::OK, Json(msgstructs::UserToken(new_token))))
            })
        })
        .await
        .map_err(tr_conv_code)
}

pub async fn refresh_token(
    State(state): State<MioState>,
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                // find token
                let mut token: user_token::ActiveModel = UserToken::find_by_id(token)
                    .one(txn)
                    .await?
                    .ok_or_else(|| {
                        MioInnerError::NotFound(Level::Debug, anyhow!("user token {token}"))
                    })?
                    .into();
                // update expiry
                token.expiry = Set(Utc::now() + chrono::Duration::days(TIMEOUT_TIME_DAY));
                token.update(txn).await?;

                Ok(StatusCode::OK)
            })
        })
        .await
        .map_err(tr_conv_code)
}

pub async fn logout(
    State(state): State<MioState>,
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    match UserToken::delete_by_id(token)
        .exec(&state.db)
        .await
        .map_err(db_err)?
        .rows_affected
    {
        0 => {
            debug!("POST /l/logout no token found for {token}");
            Err(StatusCode::NOT_FOUND)
        }
        1 => {
            debug!("POST /l/logout deleted token {token}");
            Ok(StatusCode::OK)
        }
        _long => {
            error!("POST /l/logout more than one record deleted on token {token}: {_long}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn signup(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> impl IntoResponse {
    // TODO: user config to disable signing up
    // TODO: defer this generation?
    // argon2 the password
    let passwd = auth.password().to_owned();
    let phc_string = tokio::task::block_in_place(move || {
        debug!("POST /l/signup generating phc string");
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let ret = Argon2::default()
            .hash_password(passwd.as_bytes(), &salt)
            .map_err(|err| {
                Into::<StatusCode>::into(MioInnerError::UserCreationFail(
                    Level::Error,
                    anyhow!("could not generate phc string: {err}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            })?;
        Ok::<_, StatusCode>(ret.to_string())
    })?;

    // then put into db
    state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                // setup user
                debug!("POST /l/signup transaction begin");
                // check if username exists
                if User::find()
                    .filter(user::Column::Username.eq(auth.username()))
                    .one(txn)
                    .await?
                    .is_some()
                {
                    return Err(MioInnerError::UserCreationFail(
                        Level::Debug,
                        anyhow!("found username already in db"),
                        StatusCode::CONFLICT,
                    ));
                }

                // generate uid
                let uid = loop {
                    let uid = Uuid::new_v4();
                    if User::find_by_id(uid).one(txn).await?.is_none() {
                        break uid;
                    }
                    trace!("POST /l/signup uuid collison detected: {uid}")
                };
                debug!("POST /l/signup user id generated: {uid}");

                // insert into db
                user::ActiveModel {
                    id: Set(uid),
                    username: Set(auth.username().to_owned()),
                    password: Set(phc_string),
                }
                .insert(txn)
                .await?;

                debug!("POST /l/signup created user {uid}");
                Ok(StatusCode::OK)
            })
        })
        .await
        .map_err(tr_conv_code)
}
