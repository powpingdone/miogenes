use axum::http::{
    Request,
    StatusCode,
};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use once_cell::sync::OnceCell;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{
    ConnectOptions,
    SqlitePool,
};
use std::str::FromStr;
use std::sync::Arc;
use subtasks::secret::SecretHolder;
use tokio::sync::Semaphore;

mod db;
mod endpoints;
mod error;
mod subtasks;
mod user;

use endpoints::*;
pub(crate) use crate::error::*;

// use endpoints::*; TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

#[derive(Clone)]
pub struct MioState {
    db: SqlitePool,
    lim: Arc<Semaphore>,
    lock_files: Arc<tokio::sync::RwLock<()>>,
    secret: SecretHolder,
}

// this is needed for weird axum state shenatigans
trait MioStateRegen {
    fn get_self(&self) -> MioState;
}

impl MioStateRegen for MioState {
    fn get_self(&self) -> MioState {
        self.clone()
    }
}

async fn version() -> impl IntoResponse {
    use konst::primitive::parse_u16;
    use konst::result::unwrap_ctx;

    const VSTR: mio_common::Vers =
        mio_common::Vers::new(
            unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MAJOR"))),
            unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_MINOR"))),
            unwrap_ctx!(parse_u16(env!("CARGO_PKG_VERSION_PATCH"))),
        );
    (StatusCode::OK, Json(VSTR))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: tracing
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("trace"));
    gstreamer::init()?;

    // create the main passing state
    trace!("main: creating state");
    let settings = SqliteConnectOptions::from_str("sqlite://files/music.db").unwrap().create_if_missing(true);
    let db = SqlitePool::connect_with(settings).await.expect("Could not load database: {}");
    trace!("main: migrating database");
    sqlx::migrate!().run(&db).await.unwrap();
    let state = MioState {
        db,
        lim: Arc::new(Semaphore::const_new({
            let cpus = num_cpus::get();
            if cpus <= 1 {
                cpus
            } else {
                cpus - 1
            }
        })),
        lock_files: Arc::new(tokio::sync::RwLock::const_new(())),
        secret: SecretHolder::new().await,
    };

    // setup the router
    //
    // TODO: this needs to be not static
    trace!("main: building router");
    let router =
        Router::new()
            .nest(
                "/api",
                Router::new()
                    .route("/search", get(search::search))
                    .nest("/track", track_manage::routes())
                    .nest("/query", query::routes())
                    .nest("/load", idquery::routes())
                    .nest("/folder", folders::routes()),
            )
            .route_layer(middleware::from_extractor_with_state::<user::Authenticate, _>(state.clone()))
            .nest("/user", Router::new().route("/login", get(user::login)).route("/signup", post(user::signup)))
            .route("/ver", get(version))
            .layer(axum::middleware::from_fn(log_req))
            .with_state(state.clone());

    // TODO: bind to user settings
    static BINDING: &str = "127.0.0.1:8081";
    info!("main: starting server on {BINDING}");
    Server::bind(&BINDING.parse().unwrap())
        .serve(router.into_make_service())
        .await
        .expect("server exited improperly: {}");
    trace!("main: cleaning up nicely");
    state.db.close().await;
    Ok(())
}

// small logging function that logs the method (eg, GET) and endpoint uri
async fn log_req<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    info!("{} {}", req.method(), req.uri());
    next.run(req).await
}
