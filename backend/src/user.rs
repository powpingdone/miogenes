use crate::db::uuid_serialize;
use crate::{MioInnerError, MioState, MioStateRegen};
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequestParts, Query, State};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::{Authorization, Cookie};
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Json, RequestPartsExt, TypedHeader};
use chrono::Utc;
use log::*;
use mio_common::*;
use once_cell::sync::Lazy;
use rand::prelude::*;
use sqlx::Connection;
use uuid::Uuid;

pub(crate) struct Authenticate;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticate
where
    S: Send + Sync + MioStateRegen,
{
    type Rejection = MioInnerError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // get user token from either a Authorization Bearer header or from cookies
        let state = state.get_self();
        let auth = {
            // auth header
            let authheader: Result<TypedHeader<Authorization<Bearer>>, _> = req.extract().await;
            let cookies: Result<TypedHeader<Cookie>, _> = req.extract().await;
            let raw_token = if let Ok(auth_bearer) = authheader.as_ref() {
                auth_bearer.token()
            } else if let Ok(cookies) = cookies.as_ref() {
                cookies.get("Token").ok_or_else(|| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("'Token' does not exist in cookies"),
                        StatusCode::BAD_REQUEST,
                    )
                })?
            } else {
                return Err(
                    MioInnerError::UserChallengedFail(
                        anyhow!(
                            "failed to get token as both auth and cookie were in err: (auth: {}, cookie: {})",
                            authheader.unwrap_err(),
                            cookies.unwrap_err()
                        ),
                        StatusCode::BAD_REQUEST,
                    ),
                );
            };
            auth::JWT::from_raw(raw_token.to_string())
                .decode(&state.secret.get_secret().await)
                .map_err(|err| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("invalid token: {err}"),
                        StatusCode::BAD_REQUEST,
                    )
                })?
        };

        // inject user
        if let Some(item) = req.extensions.insert(auth.claims) {
            warn!(
                "USER_INJ while injecting user: user of {} existed, replacing.",
                item.userid
            );
        }
        Ok(Authenticate)
    }
}

pub async fn login(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<(StatusCode, Json<auth::JWT>), MioInnerError> {
    let mut conn = state.db.acquire().await?;

    // get user
    let username = auth.username();
    let user = sqlx::query!("SELECT * FROM user WHERE username = ?;", username)
        .fetch_optional(&mut conn)
        .await?
        .ok_or_else(|| {
            MioInnerError::NotFound(anyhow!("Failed to find user \"{}\" in db", auth.username()))
        })?;

    // check hash
    let passwd = user.password.to_owned();
    tokio::task::block_in_place({
        move || {
            let parsed = PasswordHash::new(&passwd).map_err(|err| {
                MioInnerError::UserChallengedFail(
                    anyhow!("unable to extract phc string: {err}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
            Argon2::default()
                .verify_password(auth.password().to_owned().as_bytes(), &parsed)
                .map_err(|err| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("unable to verify password: {err}"),
                        StatusCode::UNAUTHORIZED,
                    )
                })
        }
    })?;

    // TODO: let server host specify when the logout tokens expire
    //
    // TODO: add set token header generate new token
    let token = auth::JWT::new(
        auth::JWTInner {
            userid: uuid_serialize(&user.id)?,
        },
        &state.secret.get_secret().await,
    )
    .map_err(|err| {
        MioInnerError::UserChallengedFail(
            anyhow::anyhow!("failed to generate JWT for user: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?;
    debug!(
        "GET /l/login new token generated for {}: {token:?}",
        uuid_serialize(&user.id).unwrap()
    );
    Ok((StatusCode::OK, Json(token)))
}

pub async fn signup(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    // TODO: user config to disable signing up
) -> Result<StatusCode, MioInnerError> {
    // TODO: defer this generation?
    let uname = auth.username().to_owned();
    let passwd = auth.password().to_owned();

    // argon2 the password
    let phc_string = tokio::task::block_in_place(move || {
        debug!("POST /l/signup generating phc string");
        let salt = argon2::password_hash::SaltString::generate(&mut rand::rngs::OsRng);
        let ret = Argon2::default()
            .hash_password(passwd.as_bytes(), &salt)
            .map_err(|err| {
                MioInnerError::UserCreationFail(
                    anyhow!("could not generate phc string: {err}"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })?;
        Ok::<_, MioInnerError>(ret.to_string())
    })?;

    // then put into db
    state
        .db
        .acquire()
        .await?
        .transaction(|txn| {
            Box::pin(async move {
                // setup user
                debug!("POST /l/signup transaction begin");
                if sqlx::query!("SELECT * FROM user WHERE username = ?;", uname)
                    .fetch_optional(&mut *txn)
                    .await?
                    .is_some()
                {
                    return Err(MioInnerError::UserCreationFail(
                        anyhow!("username already taken"),
                        StatusCode::CONFLICT,
                    ));
                }

                // generate uuid
                let uid = loop {
                    let uid = Uuid::new_v4();
                    if sqlx::query!("SELECT * FROM user WHERE id = ?;", uid)
                        .fetch_optional(&mut *txn)
                        .await?
                        .is_none()
                    {
                        break uid;
                    }
                };
                sqlx::query!(
                    "INSERT INTO user (id, username, password) VALUES (?,?,?);",
                    uid,
                    uname,
                    phc_string
                )
                .execute(&mut *txn)
                .await?;
                Ok(StatusCode::OK)
            })
        })
        .await
}
