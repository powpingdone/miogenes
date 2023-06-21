use crate::db::{uuid_serialize, write_transaction};
use crate::subtasks::secret::stat_secret;
use crate::{MioInnerError, MioState, MioStateRegen};
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequestParts, State};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Json, RequestPartsExt, TypedHeader};
use chrono::Utc;
use log::*;
use mio_common::*;
use std::path::PathBuf;
use uuid::Uuid;

pub(crate) struct Authenticate;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticate
where
    S: Send + Sync + MioStateRegen,
{
    type Rejection = MioInnerError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // get user token from a Authorization Bearer header
        let state = state.get_self();
        let auth = {
            // auth header
            let authheader: Result<TypedHeader<Authorization<Bearer>>, _> = req.extract().await;
            let raw_token = if let Ok(auth_bearer) = authheader.as_ref() {
                auth_bearer.token()
            } else {
                return Err(MioInnerError::UserChallengedFail(
                    anyhow!(
                        "failed to get token as auth was in err: {}",
                        authheader.unwrap_err()
                    ),
                    StatusCode::BAD_REQUEST,
                ));
            };
            auth::JWT::from_raw(raw_token.to_owned())
                .decode(&state.secret.get_secret().await)
                .map_err(|err| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("invalid token: {err}"),
                        StatusCode::UNAUTHORIZED,
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
) -> Result<impl IntoResponse, MioInnerError> {
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
    tokio::task::spawn_blocking({
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
    })
    .await??;

    // TODO: let server host specify when the logout tokens expire
    //
    // TODO: add set token header generate new token
    let token = auth::JWT::new(
        auth::JWTInner {
            userid: uuid_serialize(&user.id)?,
            exp: (Utc::now()
                + chrono::Duration::from_std(stat_secret().await).map_err(|err| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("Failed to generate exp: {err}"),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )
                })?)
            .timestamp(),
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
        "GET /user/login new token generated for {:?}: {token:?}",
        uuid_serialize(&user.id)
    );
    Ok((StatusCode::OK, Json(token)))
}

// TODO: is this a good idea to be an endpoint or to have a private thing?
pub async fn signup(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    // TODO: user config to disable signing up
) -> Result<impl IntoResponse, MioInnerError> {
    // TODO: defer this generation?
    let uname = auth.username().to_owned();
    let passwd = auth.password().to_owned();

    // argon2 the password
    let phc_string = tokio::task::spawn_blocking(move || {
        debug!("POST /user/signup generating phc string");
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
    });

    // then put into db and create dir
    let mut conn = state.db.acquire().await?;
    write_transaction(&mut conn, |txn| {
        Box::pin(async move {
            // setup user
            debug!("POST /user/signup transaction begin");
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
            let phc_string = phc_string.await??;
            sqlx::query!(
                "INSERT INTO user (id, username, password) VALUES (?,?,?);",
                uid,
                uname,
                phc_string
            )
            .execute(&mut *txn)
            .await?;

            // create the user dir if not exists
            if let Err(err) = {
                tokio::fs::create_dir(
                    [*crate::DATA_DIR.get().unwrap(), &format!("{uid}")]
                        .into_iter()
                        .collect::<PathBuf>(),
                )
                .await
            } {
                if err.kind() != std::io::ErrorKind::AlreadyExists {
                    error!("POST /user/signup failed to create user directory: {err}");
                    return Err(MioInnerError::IntIoError(anyhow!(
                        "failed to create user dir: {err}"
                    )));
                }
            }
            Ok(StatusCode::OK)
        })
    })
    .await
}

#[cfg(test)]
mod test {
    use crate::test::*;
    use axum::{
        headers::{authorization::Credentials, Authorization},
        http::{HeaderName, Method},
    };
    use mio_common::*;

    #[tokio::test]
    async fn user_auth_good() {
        let cli = client().await;
        let jwt = gen_user(&cli, "user_auth_good").await;
        jwt_header(&cli, Method::GET, "/api/auth_test", &jwt).await;
    }

    #[tokio::test]
    async fn user_auth_bad_token_bad() {
        let cli = client().await;
        let fake_jwt = auth::JWT::from_raw("a.aaaaa.aaaaaaaaaaaaaaa".to_owned());
        jwt_header(&cli, Method::GET, "/api/auth_test", &fake_jwt)
            .expect_failure()
            .await;
    }

    #[tokio::test]
    async fn user_auth_bad_basic_not_bearer() {
        let cli = client().await;
        let _ = gen_user(&cli, "basic_not_bearer").await;
        cli.get("/api/auth_get")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic("basic_not_bearer", "password")
                    .0
                    .encode(),
            )
            .expect_failure()
            .await;
    }

    // user_login_good is already proven by `gen_user`
    #[tokio::test]
    async fn user_login_bad_user_bad() {
        let cli = client().await;
        cli.get("/user/login")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic("NOT A USERNAME", "password")
                    .0
                    .encode(),
            )
            .expect_failure()
            .await;
    }

    #[tokio::test]
    async fn user_login_bad_password_bad() {
        let cli = client().await;
        let _ = gen_user(&cli, "password_bad").await;
        cli.get("/user/login")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic("password_bad", "notpassword")
                    .0
                    .encode(),
            )
            .expect_failure()
            .await;
    }

    // user_signup_good is already proven by `gen_user`
    #[tokio::test]
    async fn user_signup_bad_username_conflict() {
        let cli = client().await;
        let _ = gen_user(&cli, "username_conflict").await;
        cli.post("/usr/signup")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic("username_conflict", "password")
                    .0
                    .encode(),
            )
            .expect_failure()
            .await;
    }
}
