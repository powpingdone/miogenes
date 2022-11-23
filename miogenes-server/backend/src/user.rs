use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequest, Query, RequestParts};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Extension, Json, TypedHeader};
use chrono::{Duration, Utc};
use log::*;
use rand::rngs::OsRng;
use sea_orm::{prelude::*, *};
use uuid::Uuid;

use crate::{db_err, MioState};
use mio_common::*;
use mio_entity::{user, user_token, User, UserToken};

static TIMEOUT_TIME: i64 = 3;

pub(crate) struct Authenticate;

#[async_trait]
impl<B> FromRequest<B> for Authenticate
where
    B: Send,
{
    type Rejection = StatusCode;
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(state) = req.extract::<Extension<Arc<MioState>>>().await.unwrap();
        // get user token
        let auth = Uuid::parse_str(
            req.extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .unwrap()
                .token(),
        )
        .map_err(|err| {
            debug!("USER_INJ could not parse token: {err}");
            StatusCode::BAD_REQUEST
        })?;

        // check for existence and validity of token
        let usertoken = UserToken::find_by_id(auth)
            .one(&state.db)
            .await
            .map_err(db_err)?
            .ok_or_else(|| {
                debug!("USER_INJ usertoken not found");
                StatusCode::UNAUTHORIZED
            })?;
        if usertoken.expiry > Utc::now() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        // inject user
        let user = User::find_by_id(usertoken.user_id)
            .one(&state.db)
            .await
            .map_err(db_err)?
            .ok_or_else(|| {
                error!("USER_INJ usertoken found, but no user found");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        if let Some(item) = req.extensions_mut().insert(user) {
            warn!(
                "USER_INJ warning while injecting user: user of {} already existed. replacing.",
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
    // get user
    let user = User::find()
        .filter(user::Column::Username.eq(auth.username()))
        .one(&state.db)
        .await
        .map_err(db_err)?
        .ok_or_else(|| {
            debug!(
                "GET /l/login failed to find user \"{}\" in index",
                auth.username()
            );
            StatusCode::UNAUTHORIZED
        })?;

    // check hash
    let passwd = user.password.to_owned();
    tokio::task::spawn_blocking({
        move || {
            let parsed = PasswordHash::new(&passwd).map_err(|err| {
                error!("GET /l/login unable to extract phc string: {err}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            Argon2::default()
                .verify_password(auth.password().to_owned().as_bytes(), &parsed)
                .map_err(|err| {
                    debug!("GET /l/login unable to verify password: {err}");
                    StatusCode::UNAUTHORIZED
                })
        }
    })
    .await
    .map_err(|err| {
        error!("GET /l/login task failed to start: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })??;

    // generate new token
    let new_token = Uuid::new_v4();
    // TODO: let server host specify when logout tokens expire
    let expiry = Utc::now() + chrono::Duration::days(TIMEOUT_TIME);
    user_token::Entity::insert(UserToken::insert(user_token::ActiveModel {
        id: Set(new_token),
        expiry: Set(expiry),
        user_id: Set(user.id),
    }))
    .exec(&state.db)
    .await
    .map_err(db_err)?;

    debug!(
        "GET /l/login new token generated for {}: {new_token}, expires {expiry}",
        user.id
    );
    Ok((StatusCode::OK, Json(msgstructs::UserToken(new_token))))
}

pub async fn refresh_token(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    state
        .db
        .transaction(|txn| {
            Box::pin(async move {
                let mut token: user_token::ActiveModel = UserToken::find_by_id(token)
                    .one(txn)
                    .await
                    .map_err(db_err)?
                    .ok_or_else(|| {
                        debug!("POST /l/login no user token found");
                        StatusCode::NOT_FOUND
                    })?
                    .into();
                token.expiry = Set(Utc::now() + chrono::Duration::days(TIMEOUT_TIME));
                token.update(txn).await.map_err(db_err)?;

                Ok::<_, StatusCode>(StatusCode::OK)
            })
        })
        .await
        .map_err(|err| match err {
            TransactionError::Connection(err) => db_err(err),
            TransactionError::Transaction(err) => err,
        })
}

pub async fn logout(
    Extension(state): Extension<Arc<crate::MioState>>,
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
    Extension(state): Extension<Arc<crate::MioState>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<StatusCode, StatusCode> {
    static HOLD: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

    // TODO: user config to disable signing up
    // argon2 the password
    let passwd = auth.password().to_owned();
    let phc_string = tokio::task::block_in_place(move || {
        debug!("POST /l/signup generating phc string");
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let ret = Argon2::default()
            .hash_password(passwd.as_bytes(), &salt)
            .map_err(|err| {
                error!("POST /l/signup could not generate phc string: {err}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Ok::<_, StatusCode>(ret.to_string())
    })?;

    // then, setup user
    debug!("POST /l/signup acquire HOLD");
    let lock = HOLD.acquire().await.map_err(|err| {
        error!("POST /l/signup semaphore failed to acquire: {err}",);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let ret = tokio::task::block_in_place(move || {
        debug!("POST /l/signup writing out to db");
        let idxtree = state
            .db
            .open_tree(TopLevel::IndexUsernameToUser.table())
            .unwrap();
        let usertree = state.db.open_tree(TopLevel::User.table()).unwrap();
        let uid = (&idxtree, &usertree)
            .transaction(move |(idxtree, usertree)| {
                debug!("POST /l/signup transaction begin");
                // check if username exists
                if idxtree.get(auth.username().as_bytes()).unwrap().is_some() {
                    abort(StatusCode::CONFLICT)?;
                }

                // setup user
                let uid = loop {
                    let uid = Uuid::new_v4();
                    if usertree.get(uid.as_bytes()).unwrap().is_none() {
                        break uid;
                    }
                };
                usertree.insert(
                    uid.as_bytes(),
                    User::generate(auth.username().to_owned(), phc_string.clone()),
                )?;
                idxtree.insert(auth.username().as_bytes(), uid.as_bytes())?;

                Ok(uid)
            })
            .map_err(|err| match err {
                TransactionError::Abort(err) => err,
                TransactionError::Storage(err) => panic!("db failure: {err}"),
            })?;

        debug!("POST /l/signup created user {uid}");
        Ok(StatusCode::OK)
    });
    drop(lock);
    ret
}
