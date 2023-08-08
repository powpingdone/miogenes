use crate::db::{uuid_serialize, write_transaction};
use crate::{MioInnerError, MioState, MioStateRegen};
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract::{FromRequestParts, State};
use axum::headers::authorization::{Basic, Bearer};
use axum::headers::Authorization;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{async_trait, Extension, Json, RequestPartsExt, TypedHeader};
use chrono::Utc;
use log::*;
use mio_common::*;
use sqlx::SqliteConnection;
use std::path::PathBuf;
use uuid::Uuid;

const SECRET_SIZE: usize = 1024;

pub(crate) struct Authenticate;

#[async_trait]
impl<S> FromRequestParts<S> for Authenticate
where
    S: Send + Sync + MioStateRegen,
{
    type Rejection = MioInnerError;

    async fn from_request_parts(req: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let state = state.get_self();

        // get user token from a Authorization Bearer header
        let authheader: Result<TypedHeader<Authorization<Bearer>>, _> = req.extract().await;
        let raw_token = if let Ok(auth_bearer) = authheader.as_ref() {
            auth_bearer.token()
        } else {
            return Err(MioInnerError::UserChallengedFail(
                anyhow!(
                    "Failed to get token as auth was in err: {}",
                    authheader.unwrap_err()
                ),
                StatusCode::BAD_REQUEST,
            ));
        };
        let potent_token = auth::JWT::from_raw(raw_token.to_owned());

        // get secrets
        let potent_id = potent_token
            .whois()
            .map_err(|x| {
                debug!("USER_INJ token could not whois: {x}");
                MioInnerError::UserChallengedFail(
                    anyhow!("Invalid auth token"),
                    StatusCode::UNAUTHORIZED,
                )
            })?
            .userid;
        let mut conn = state.db.acquire().await?;
        let now = Utc::now().timestamp();
        let secrets = sqlx::query!(
            "SELECT secret 
            FROM auth_keys 
            WHERE id = ? AND expiry > ? 
            ORDER BY expiry ASC;",
            potent_id,
            now
        )
        .fetch_all(&mut *conn)
        .await?;

        // check all keys
        let tasks = secrets
            .into_iter()
            .map(|x| {
                tokio::task::spawn_blocking({
                    let potent_token = potent_token.clone();
                    move || potent_token.decode(&x.secret)
                })
            })
            .collect::<Vec<_>>();
        let mut auth = None;
        for task in tasks {
            match task.await? {
                Ok(ret) => {
                    auth = Some(ret);
                    break;
                }
                Err(err) => debug!("USER_INJ could not auth token: {err}"),
            }
        }
        if auth.is_none() {
            return Err(MioInnerError::UserChallengedFail(
                anyhow!("Invalid auth token"),
                StatusCode::UNAUTHORIZED,
            ));
        }

        // inject user
        if let Some(item) = req.extensions.insert(auth.unwrap().claims) {
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
    write_transaction(&mut conn, |txn| {
        Box::pin(async move {
            // get user
            let username = auth.username();
            let user = sqlx::query!("SELECT * FROM user WHERE username = ?;", username)
                .fetch_optional(&mut *txn)
                .await?
                .ok_or_else(|| {
                    MioInnerError::UserChallengedFail(
                        anyhow!("Unable to verify user on server"),
                        StatusCode::UNAUTHORIZED,
                    )
                })?;
            let userid = uuid_serialize(&user.id)?;

            // check hash
            let passwd = user.password.to_owned();
            tokio::task::spawn_blocking({
                move || {
                    let parsed = PasswordHash::new(&passwd).map_err(|err| {
                        MioInnerError::UserChallengedFail(
                            anyhow!("Unable to extract phc string: {err}"),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        )
                    })?;
                    Argon2::default()
                        .verify_password(auth.password().to_owned().as_bytes(), &parsed)
                        .map_err(|_| {
                            MioInnerError::UserChallengedFail(
                                anyhow!("Unable to verify user on server"),
                                StatusCode::UNAUTHORIZED,
                            )
                        })
                }
            })
            .await??;

            // generate new token
            let token = create_new_token(&mut *txn, userid).await?;
            debug!("GET /user/login new token generated for {userid:?}");
            Ok((StatusCode::OK, Json(token)))
        })
    })
    .await
}

// create new token
pub async fn new_token(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
) -> Result<(StatusCode, Json<auth::JWT>), MioInnerError> {
    let mut conn = state.db.acquire().await?;
    Ok((
        StatusCode::OK,
        Json(create_new_token(&mut conn, userid).await?),
    ))
}

// util function for creating new secret in db
async fn create_new_token(
    conn: &mut SqliteConnection,
    userid: Uuid,
) -> Result<auth::JWT, MioInnerError> {
    // TODO: let server host specify when the logout tokens expire
    let secret: [u8; SECRET_SIZE] = rand::random();
    let slice = secret.as_slice();
    let exp = Utc::now()
        .checked_add_days(chrono::Days::new(7))
        .unwrap()
        .timestamp();
    sqlx::query!(
        "INSERT INTO auth_keys (id, expiry, secret) VALUES (?,?,?);",
        userid,
        exp,
        slice
    )
    .execute(conn)
    .await?;
    auth::JWT::new(auth::JWTInner { userid, exp }, &secret).map_err(|err| {
        MioInnerError::UserChallengedFail(
            anyhow::anyhow!("failed to generate JWT for user: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })
}

// TODO: user config to disable signing up
//
// TODO: is this a good idea to be an endpoint or to have a private thing?
pub async fn signup(
    State(state): State<MioState>,
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
) -> Result<impl IntoResponse, MioInnerError> {
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
                phc_string.abort();
                drop(phc_string);
                return Err(MioInnerError::UserCreationFail(
                    anyhow!("Username already taken."),
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
                        "Failed to create user dir: {err}"
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

    #[tokio::test]
    async fn user_refresh_good() {
        let cli = client().await;
        let jwt = gen_user(&cli, "user_refresh_good").await;
        let new_jwt = jwt_header(&cli, Method::PATCH, "/user/refresh", &jwt)
            .await
            .json::<auth::JWT>();
        jwt_header(&cli, Method::GET, "/api/auth_test", &new_jwt).await;
    }
}
