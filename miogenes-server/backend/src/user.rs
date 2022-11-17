use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequest, Query, RequestParts};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Extension, Json, TypedHeader};
use chrono::Duration;
use log::*;
use rand::rngs::OsRng;
use sled::transaction::{abort, TransactionError};
use sled::Transactional;
use uuid::Uuid;

use crate::db::{self, DbTable, Index, TopLevel, User, UserToken};
use crate::MioState;
use mio_common::*;

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
        let auth = Uuid::parse_str(
            req.extract::<TypedHeader<Authorization<Bearer>>>()
                .await
                .unwrap()
                .token(),
        )
        .map_err(|err| {
            debug!("could not parse token: {err}");
            StatusCode::BAD_REQUEST
        })?;
        let user: Index<UserToken> = {
            let user: Option<Index<UserToken>> = state
                .db
                .open_tree(TopLevel::UserToken.table())
                .unwrap()
                .get(auth.as_bytes())
                .unwrap()
                .map(|ret| Index::new(auth, &ret).unwrap());
            if let Some(ret) = user {
                ret
            } else {
                debug!("USER_INJ token not found: {}", auth);
                return Err(StatusCode::UNAUTHORIZED);
            }
        };

        let user = Index::<User>::new(
            user.inner().user(),
            &state
                .db
                .open_tree(TopLevel::User.table())
                .unwrap()
                .get(user.inner().user())
                .unwrap()
                .ok_or_else(|| {
                    error!(
                        "USER_INJ could not find user with token {}",
                        user.inner().user()
                    );
                    StatusCode::INTERNAL_SERVER_ERROR
                })?,
        )
        .map_err(|err| {
            error!("USER_INJ failed to serialize user: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        if let Some(item) = req.extensions_mut().insert(user) {
            warn!(
                "USER_INJ warning while injecting user: user of {:?} already existed. replacing.",
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
        let tree = state
            .db
            .open_tree(TopLevel::IndexUsernameToUser.table())
            .unwrap();
        let utree = state.db.open_tree(TopLevel::User.table()).unwrap();

        if let Some(uid) = tree.get(auth.username().as_bytes()).unwrap() {
            Index::new(
                Uuid::from_slice(&uid).unwrap(),
                &utree.get(uid).unwrap().unwrap(),
            )
            .unwrap()
        } else {
            debug!(
                "GET /l/login failed to find user \"{}\" in index",
                auth.username()
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // check hash
    let passwd = user.inner().password().to_owned();
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

    // gen new token
    let new_token = Uuid::new_v4();
    // TODO: let server host specify when logout tokens expire
    let expiry = chrono::Utc::now() + chrono::Duration::days(TIMEOUT_TIME);
    state
        .db
        .open_tree(TopLevel::UserToken.table())
        .unwrap()
        .insert(new_token, UserToken::generate(user.id(), expiry))
        .map_err(|err| {
            error!("GET /l/login failed to insert new token: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    debug!(
        "GET /l/login new token generated for {}: {new_token}, expires {expiry}",
        user.id()
    );
    Ok((StatusCode::OK, Json(msgstructs::UserToken(new_token))))
}

pub async fn refresh_token(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(msgstructs::UserToken(token)): Query<msgstructs::UserToken>,
) -> impl IntoResponse {
    #[derive(Debug)]
    enum ErrorOut {
        NoUserFound,
        DeserializationErr,
        TimeExpired,
    }

    let ret = state
        .db
        .open_tree(TopLevel::UserToken.table())
        .unwrap()
        .transaction(|tx_db| {
            let timeup: Result<Index<db::UserToken>, _> = Index::new(token, &{
                let x = tx_db.get(token)?;
                if x.is_none() {
                    abort(ErrorOut::NoUserFound)?;
                }
                x.unwrap()
            });
            if let Err(ref err) = timeup {
                error!("POST /l/login deserialization error: {err}");
                abort(ErrorOut::DeserializationErr)?;
            }
            let mut timeup = timeup.unwrap();

            if !timeup.inner().is_expired() {
                timeup
                    .inner_mut()
                    .push_forward(Duration::days(TIMEOUT_TIME));
            } else {
                abort(ErrorOut::TimeExpired)?;
            }

            tx_db.insert(token.as_bytes(), timeup.decompose().unwrap())?;

            Ok(())
        });

    if let Err(err) = ret {
        debug!("POST /l/login error encountered: {err:?}");
        match err {
            sled::transaction::TransactionError::Abort(x) => match x {
                ErrorOut::NoUserFound => StatusCode::NOT_FOUND,
                ErrorOut::DeserializationErr => StatusCode::INTERNAL_SERVER_ERROR,
                ErrorOut::TimeExpired => StatusCode::GONE,
            },
            sled::transaction::TransactionError::Storage(err) => {
                panic!("encountered db problem: {err}")
            }
        }
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

pub async fn signup(
    Extension(state): Extension<Arc<crate::MioState>>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<StatusCode, StatusCode> {
    static HOLD: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

    // TODO: user config to disable signing up
    // argon2 the password
    let passwd = auth.password().to_owned();
    let phc_string = tokio::task::spawn_blocking(move || {
        debug!("POST /l/signup generating phc string");
        let salt = argon2::password_hash::SaltString::generate(&mut OsRng);
        let ret = Argon2::default()
            .hash_password(passwd.as_bytes(), &salt)
            .map_err(|err| {
                error!("POST /l/signup could not generate phc string: {err}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        Ok::<_, StatusCode>(ret.to_string())
    })
    .await
    .map_err(|err| {
        error!("POST /l/signup argon passwd generation failed: {err}");
        StatusCode::INTERNAL_SERVER_ERROR
    })??;

    // meanwhile, setup user
    debug!("POST /l/signup acquire HOLD");
    let lock = HOLD.acquire().await.map_err(|err| {
        error!("POST /l/signup semaphore failed to acquire: {err}",);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let ret = tokio::task::spawn_blocking(move || {
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
    })
    .await
    .map_or_else(
        |err| {
            error!("POST /l/signup failed to start db task: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
        |ret| ret,
    );
    drop(lock);
    ret
}
