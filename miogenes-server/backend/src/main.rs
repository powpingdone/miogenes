use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use mio_migration::{Migrator, MigratorTrait};
use once_cell::sync::OnceCell;
use sea_orm::{Database, DatabaseConnection, DbErr, TransactionError};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::Semaphore;
use uuid::Uuid;

mod endpoints;

use endpoints::*;

mod subtasks;

use subtasks::*;

mod db;
mod user;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

// transaction errors used in the server
#[derive(Debug, Error)]
pub enum MioInnerError {
    #[error("could not find: `{1}`")]
    NotFound(Level, anyhow::Error),
    #[error("DATABASE ERROR: `{1}`")]
    DbError(Level, anyhow::Error),
    #[error("user challenge failure: `{1}`")]
    UserChallengedFail(Level, anyhow::Error, StatusCode),
    #[error("user creation failure: `{1}`")]
    UserCreationFail(Level, anyhow::Error, StatusCode),
}

impl From<sea_orm::DbErr> for MioInnerError {
    fn from(err: sea_orm::DbErr) -> Self {
        MioInnerError::DbError(Level::Error, anyhow::Error::new(err))
    }
}

#[allow(clippy::from_over_into)]
impl Into<StatusCode> for MioInnerError {
    fn into(self) -> StatusCode {
        // log errors
        log!(
            match self {
                MioInnerError::NotFound(lvl, _)
                | MioInnerError::DbError(lvl, _)
                | MioInnerError::UserChallengedFail(lvl, _, _)
                | MioInnerError::UserCreationFail(lvl, _, _) => lvl,
            },
            "{self}"
        );

        // return status
        match self {
            MioInnerError::NotFound(..) => StatusCode::NOT_FOUND,
            MioInnerError::DbError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            MioInnerError::UserChallengedFail(_, _, code) => code,
            MioInnerError::UserCreationFail(_, _, code) => code,
        }
    }
}

pub fn db_err(err: DbErr) -> StatusCode {
    MioInnerError::from(err).into()
}

// change a transaction error to a StatusCode
pub fn tr_conv_code(err: TransactionError<MioInnerError>) -> StatusCode {
    match err {
        TransactionError::Connection(err) => db_err(err),
        TransactionError::Transaction(err) => err.into(),
    }
}

#[derive(Clone)]
pub struct MioState {
    db: DatabaseConnection,
    proc_tracks_tx: UnboundedSender<(Uuid, Uuid, String)>,
    lim: Arc<Semaphore>,
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
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("trace"));
    gstreamer::init()?;

    // create the main passing state
    trace!("main: creating state");
    let db = Database::connect("sqlite:file:./files/db?mode=rwc").await?;
    Migrator::up(&db, None).await?;
    let (proc_tracks_tx, proc_tracks_rx) = unbounded_channel();
    let state = MioState {
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
    };

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
                .nest("/query", query::routes())
                .nest("/load", idquery::routes()),
        )
        .route_layer(
            middleware::from_extractor_with_state::<user::Authenticate, _>(state.db.clone()),
        )
        .merge(axum_extra::routing::SpaRouter::new("/assets", STATIC_DIR))
        .nest(
            "/l",
            Router::new()
                .route("/login", get(user::login).post(user::refresh_token))
                .route("/logout", post(user::logout))
                .route("/signup", post(user::signup)),
        )
        .layer(axum::middleware::from_fn(log_req))
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
