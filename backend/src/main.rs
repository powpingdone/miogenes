use axum::http::{
    Request,
    StatusCode,
    HeaderName,
};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use mio_migration::{
    Migrator,
    MigratorTrait,
};
use once_cell::sync::OnceCell;
use sea_orm::{
    Database,
    DatabaseConnection,
};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tower_http::services::{
    ServeDir,
    ServeFile,
};

mod endpoints;
mod subtasks;
mod db;
mod user;
mod error;

pub(crate) use crate::error::*;
use endpoints::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

#[derive(Clone)]
pub struct MioState {
    db: DatabaseConnection,
    lim: Arc<Semaphore>,
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
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("trace"));
    gstreamer::init()?;

    // create the main passing state
    trace!("main: creating state");
    let db = Database::connect("sqlite:file:./files/db?mode=rwc").await?;
    Migrator::up(&db, None).await?;
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
    };

    // spin subtasks
    trace!("main: spinning subtasks");
    let subtasks: Vec<tokio::task::JoinHandle<()>> = vec![];
    trace!("main: building router");

    // TODO: this needs to be not static
    static STATIC_DIR: &str = "./dist";
    // setup the router
    let router =
        Router::new()
            .nest(
                "/api",
                Router::new()
                    .route("/ver", get(version))
                    .route("/search", get(search::search))
                    .nest("/track", track_manage::routes())
                    .nest("/query", query::routes())
                    .nest("/load", idquery::routes()),
            )
            .route_layer(middleware::from_extractor_with_state::<user::Authenticate, _>(state.db.clone()))
            .nest_service("/assets", ServeDir::new(STATIC_DIR))
            .nest(
                "/l",
                Router::new()
                    .route("/login", get(user::login).post(user::refresh_token))
                    .route("/logout", post(user::logout))
                    .route("/signup", post(user::signup)),
            )
            .layer(axum::middleware::from_fn(log_req))
            .fallback_service(ServeFile::new(&format!("{STATIC_DIR}/index.html")))
            .with_state(state);

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
