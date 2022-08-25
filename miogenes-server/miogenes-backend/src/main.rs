use actix_web::http::header::ContentType;
use actix_web::middleware::Logger;
use actix_web::*;
use entity_self::prelude::*;
use once_cell::sync::OnceCell;
use sea_orm::prelude::Uuid;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;

mod endpoints;
use endpoints::*;
mod subtasks;
use subtasks::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&'static str> = OnceCell::with_value("./files/");

macro_rules! login_check {
    ($db:expr, $key:expr) => {{
        let db = $db.into_inner();
        let key = $key.into_inner();
        let userid = key.check(&db).await;
        if let Err(ret) = userid {
            return ret;
        }
        let userid = userid.unwrap();
        (db, userid)
    }};
}

pub(crate) use login_check;

#[serde_as]
#[derive(Deserialize)]
pub struct User {
    #[serde(rename = "u")]
    username: Uuid,
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    #[serde(rename = "h")]
    password: [u8; 32],
}

#[derive(Serialize)]
pub struct MioError<'a> {
    msg: &'a str,
}

impl ToString for MioError<'_> {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

impl User {
    async fn check(&self, db: &DatabaseConnection) -> Result<Uuid, HttpResponse> {
        let check = UserTable::find_by_id(self.username).one(db).await;
        // if db didn't error out
        if let Ok(check) = check {
            if let Some(check) = check {
                // if the sha256 hash matches
                if check.password == self.password {
                    Ok(self.username)
                } else {
                    Err(HttpResponse::Unauthorized()
                        .content_type(ContentType::json())
                        .body(
                            MioError {
                                msg: "invalid password",
                            }
                            .to_string(),
                        ))
                }
            } else {
                Err(HttpResponse::Unauthorized()
                    .content_type(ContentType::json())
                    .body(
                        MioError {
                            msg: "invalid user id",
                        }
                        .to_string(),
                    ))
            }
        } else {
            Err(HttpResponse::InternalServerError()
                .content_type(ContentType::json())
                .body(
                    MioError {
                        msg: "database error",
                    }
                    .to_string(),
                ))
        }
    }
}

#[get("/ver")]
async fn version() -> impl Responder {
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

    web::Json(VSTR)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use migration::{Migrator, MigratorTrait};
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // TODO: pick this up from config file
    let db = Arc::new(
        sea_orm::Database::connect("postgres://user:password@127.0.0.1:5432/db")
            .await
            .expect("Failed to connect to db: {}"),
    );
    Migrator::up(db.as_ref(), None).await?;

    // spin subtasks
    let (tr_up_tx, tr_up_rx) = unbounded_channel();
    let subtasks = [tokio::spawn({
        let db = db.clone();
        async move { track_upload::track_upload_server(db, tr_up_rx).await }
    })];

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(db.clone())
            .app_data(tr_up_tx.clone())
            .service(version)
            .service(heartbeat::heartbeat)
            .service(web::scope("/track").configure(track::routes))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    for subtask in subtasks {
        subtask.await.unwrap();
    }

    Ok(())
}


