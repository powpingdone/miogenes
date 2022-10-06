use axum::body::{boxed, Body};
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;
use serde_with::serde_as;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Semaphore;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;
use uuid::Uuid;

mod endpoints;
use endpoints::*;
mod subtasks;
use subtasks::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

async fn login_check(db: (), key: User) -> Result<Uuid, (StatusCode, Json<MioError>)> {
    // TODO: use axum-login(?)
    trace!("login_check: login check for {key:?}");
    let userid = key.check(&db).await;
    if userid.is_err() {
        info!(
            "login_check: invalid login for {}: {userid:?}",
            key.username
        );
    } else {
        trace!("login_check: login good for {}", key.username);
    }
    userid
}

#[derive(Clone)]
pub struct MioState {
    db: (),
    proc_tracks_tx: UnboundedSender<(Uuid, Uuid, String)>,
    lim: Arc<Semaphore>,
}

#[serde_as]
#[derive(Deserialize, Debug)]
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
    async fn check(&self, db: &()) -> Result<Uuid, (StatusCode, axum::Json<MioError>)> {
        trace!("user::check: query DB for user");
        Ok(Uuid::nil())
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
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    gstreamer::init()?;

    // TODO: pick this up from config file
    static DB_URI: &str = "";
    let db = ();
    info!("main: performing migrations for DB");

    // spin subtasks
    trace!("main: spinning subtasks");
    let (proc_tracks_tx, proc_tracks_rx) = unbounded_channel();
    let state = Arc::new(MioState {
        db: db.clone(),
        proc_tracks_tx,
        lim: Arc::new(Semaphore::const_new(num_cpus::get())),
    });
    let subtasks = [tokio::spawn({
        let state = state.clone();
        async move { subtasks::track_upload::track_upload_server(state, proc_tracks_rx).await }
    })];

    trace!("main: building router");
    // TODO: this needs to be not static
    static STATIC_DIR: &str = "./dist";
    let router = Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/ver", get(version))
                .route("/search", get(search::search))
                .nest("/track", track_manage::routes())
                .nest("/query", query::routes())
                .layer(Extension(state))
                .layer(axum::middleware::from_fn(log_req)),
        )
        .merge(axum_extra::routing::SpaRouter::new("/assets", STATIC_DIR));
    // TODO: bind to user settings
    static BINDING: &str = "127.0.0.1:8080";
    info!("main: starting server on {BINDING}");
    Server::bind(&BINDING.parse().unwrap())
        .serve(router.into_make_service())
        .await?;

    trace!("main: cleaning up nicely");
    for subtask in subtasks {
        subtask.await.expect("could not join servers");
    }

    Ok(())
}

// small logging function that logs the method (eg, GET) and endpoint uri
async fn log_req<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    info!("{} {}", req.method(), req.uri());
    next.run(req).await
}
