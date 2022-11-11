use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use once_cell::sync::OnceCell;
use serde::Serialize;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Semaphore;
use uuid::Uuid;

use std::sync::Arc;

mod endpoints;
use endpoints::*;
mod subtasks;
use subtasks::*;
mod user;
use user::*;
mod db;
use db::migrate;

use crate::db::DbTable;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

#[derive(Clone)]
pub struct MioState {
    db: sled::Db,
    proc_tracks_tx: UnboundedSender<(Uuid, Uuid, String)>,
    lim: Arc<Semaphore>,
}

#[derive(Debug, Serialize)]
pub struct MioError {
    msg: String,
}

impl MioError {
    fn i_s_e() -> Self {
        Self {
            msg: "internal server error".to_owned(),
        }
    }
}

async fn version() -> impl IntoResponse {
    use konst::primitive::parse_u16;
    use konst::result::unwrap_ctx;

    const VSTR: mio_common::Vers = mio_common::Vers::new(
        unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
        unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
        unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
    );

    (StatusCode::OK, Json(VSTR))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    gstreamer::init()?;

    // create the main passing state
    trace!("main: creating state");
    let db =
        sled::open(format!("{}/db", DATA_DIR.get().unwrap())).expect("could not open database: {}");
    migrate(&db);
    db.open_tree(crate::db::TopLevel::User.table()).unwrap().insert(uuid::uuid!("474e9c8a-3cb9-438e-9896-ded5e77fde22").as_bytes(), b"{\"username\":\"beppy\",\"password\":\"$argon2id$v=19$m=16,t=2,p=1$WkYxdjltaHpnMDV6Zng5dQ$BfaUJfTMMsE+hLzWKee6aQ\"}".as_slice()).unwrap();
    let (proc_tracks_tx, proc_tracks_rx) = unbounded_channel();
    let state = Arc::new(MioState {
        db,
        proc_tracks_tx,
        lim: Arc::new(Semaphore::const_new({
            let cpus = num_cpus::get();
            if cpus <= 1 {
                cpus
            } else {
                cpus - 1
            }
        })),
    });

    // spin subtasks
    trace!("main: spinning subtasks");
    let subtasks = [tokio::spawn({
        let state = state.clone();
        async move { track_upload::track_upload_server(state, proc_tracks_rx).await }
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
                .nest("/query", query::routes()),
        )
        .route_layer(middleware::from_extractor::<user::Authenticate>())
        .merge(axum_extra::routing::SpaRouter::new("/assets", STATIC_DIR))
        .nest(
            "/l",
            Router::new()
                .route("/login", get(user::login).post(user::refresh_token))
                .route("/logout", post(user::logout))
                .route("/signup", post(user::signup)),
        )
        .layer(axum::middleware::from_fn(log_req))
        .layer(Extension(state));
    // TODO: bind to user settings
    static BINDING: &str = "127.0.0.1:8081";
    info!("main: starting server on {BINDING}");
    Server::bind(&BINDING.parse().unwrap())
        .serve(router.into_make_service())
        .await
        .expect("server exited improperly: {}");

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
