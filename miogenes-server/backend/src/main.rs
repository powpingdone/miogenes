use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use axum_login::RequireAuthorizationLayer;
use axum_login::axum_sessions::SessionLayer;
use axum_login::axum_sessions::async_session::MemoryStore;
use gstreamer::glib;
use log::*;
use once_cell::sync::OnceCell;
use path_absolutize::Absolutize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use surrealdb::{Datastore, Session};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::sync::{Semaphore, RwLock};
use uuid::Uuid;

mod endpoints;
use endpoints::*;
mod subtasks;
use subtasks::*;
mod user;
use user::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value("./files/");

#[derive(Clone)]
pub struct MioState {
    db: Arc<Datastore>,
    sess: Session,
    proc_tracks_tx: UnboundedSender<(Uuid, Uuid, String)>,
    lim: Arc<Semaphore>,
    users: Arc<RwLock<HashMap<String, User>>>,
}

#[derive(Debug, Serialize)]
pub struct MioError {
    msg: String,
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
    let db = Arc::new(
        Datastore::new(&glib::filename_to_uri(
            Path::new(&format!("{}/db", DATA_DIR.get().unwrap())).absolutize()?,
            None,
        )?)
        .await?,
    );
    let (proc_tracks_tx, proc_tracks_rx) = unbounded_channel();
    let state = Arc::new(MioState {
        db: db.clone(),
        sess: Session::for_kv(),
        proc_tracks_tx,
        lim: Arc::new(Semaphore::const_new(num_cpus::get())),
        users: Arc::new(RwLock::new(HashMap::default())),
    });

    // create the user state
    trace!("main: setting up users");
    let secret: [u8; 64] = rand::random();
    let session_layer = SessionLayer::new(MemoryStore::new(), &secret).with_secure(false);
    debug!("main: loading users");
    let users = state.db.execute("SELECT * FROM user;", &state.sess, None, false).await.unwrap();
    for user in users {
        let user = user.result.unwrap();
        println!("{user:?}");
        todo!();
    }

    
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
                .layer(Extension(state)),
        )
        .route_layer(RequireAuthorizationLayer::<User>::login())
        .merge(axum_extra::routing::SpaRouter::new("/assets", STATIC_DIR))
        .layer(axum::middleware::from_fn(log_req))
        .layer(session_layer);
    // TODO: bind to user settings
    static BINDING: &str = "127.0.0.1:8081";
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
