use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use entity_self::prelude::*;
use once_cell::sync::OnceCell;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

extern crate ffmpeg_next as ffmpeg;

mod endpoints;
use endpoints::*;
mod subtasks;
use subtasks::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&'static str> = OnceCell::with_value("./files/");

async fn login_check(
    db: Arc<DatabaseConnection>,
    key: User,
) -> Result<Uuid, (StatusCode, Json<MioError>)> {
    let userid = key.check(&db).await;
    if let Err(ret) = userid {
        return Err(ret);
    }
    let userid = userid.unwrap();
    Ok(userid)
}

#[derive(Clone)]
pub struct MioState {
    db: Arc<DatabaseConnection>,
    proc_tracks_tx: UnboundedSender<(Uuid, Uuid, String)>,
}

#[serde_as]
#[derive(Deserialize)]
pub struct User {
    #[serde(rename = "u")]
    username: Uuid,
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(rename = "h")]
    password: [u8; 32],
}

#[derive(Debug, Serialize)]
pub struct MioError {
    msg: String,
}

impl User {
    async fn check(
        &self,
        db: &DatabaseConnection,
    ) -> Result<Uuid, (StatusCode, axum::Json<MioError>)> {
        let check = UserTable::find_by_id(self.username).one(db).await;
        // if db didn't error out
        if let Ok(check) = check {
            if let Some(check) = check {
                // if the sha256 hash matches
                if check.password == self.password {
                    Ok(self.username)
                } else {
                    Err((
                        StatusCode::UNAUTHORIZED,
                        Json(MioError {
                            msg: "invalid password".to_owned(),
                        }),
                    ))
                }
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    Json(MioError {
                        msg: "invalid user id".to_owned(),
                    }),
                ))
            }
        } else {
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(MioError {
                    msg: "database error".to_owned(),
                }),
            ))
        }
    }
}

async fn version() -> impl IntoResponse {
    use konst::primitive::parse_u16;
    use konst::result::unwrap_ctx;

    #[derive(Serialize)]
    struct Vers {
        major: u16,
        minor: u16,
        patch: u16,
    }

    const VSTR: Vers = Vers {
        major: unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
        minor: unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
        patch: unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
    };

    (StatusCode::OK, Json(VSTR))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use migration::{Migrator, MigratorTrait};
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    ffmpeg::init()?;

    // setup dirs
    for i in ["track", "albumart"] {
        tokio::fs::create_dir(format!("{}/{i}", DATA_DIR.get().expect("DATA_DIR not set")))
            .await
            .or_else(|err| {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    return Ok(());
                }
                Err(err)
            })?
    }

    // TODO: pick this up from config file
    let db = Arc::new(
        sea_orm::Database::connect("postgres://user:password@127.0.0.1:5432/db")
            .await
            .expect("Failed to connect to db: {}"),
    );
    Migrator::up(db.as_ref(), None).await?;

    // spin subtasks
    let (proc_tracks_tx, proc_tracks_rx) = unbounded_channel();
    let subtasks = [tokio::spawn({
        let db = db.clone();
        async move { track_upload::track_upload_server(db, proc_tracks_rx).await }
    })];

    let state = Arc::new(MioState {
        db: db.clone(),
        proc_tracks_tx,
    });

    let router = Router::new()
        .route("/ver", get(version))
        .route("/hb", get(heartbeat::heartbeat))
        .nest("/track", track::routes())
        .layer(Extension(state));

    Server::bind(&"127.0.0.1:8080".parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    for subtask in subtasks {
        subtask.await.unwrap();
    }

    Ok(())
}
