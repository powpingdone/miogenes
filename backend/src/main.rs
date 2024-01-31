use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::net::TcpListener;

mod db;
mod endpoints;
mod env;
mod error;
mod subtasks;
mod user;

pub(crate) use crate::env::*;
pub(crate) use crate::error::*;
use endpoints::*;

#[derive(Clone, Debug)]
pub struct MioState {
    db: SqlitePool,
    lock_files: Arc<tokio::sync::RwLock<()>>,
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

#[tracing::instrument]
pub async fn get_version() -> impl IntoResponse {
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
    init_from_env().await;
    console_subscriber::init();
    gstreamer::init()?;

    // create the main passing state
    let state = gen_state().await;

    // setup the router
    trace!("main: building router");
    let router = gen_public_router(state.clone());

    let addr = SocketAddr::new(*IP_ADDR.get().unwrap(), *PORT.get().unwrap());
    info!("main: starting server on {addr}");
    let socket = TcpListener::bind(addr)
        .await
        .expect("failed to bind to addr : {}");
    let (tx_die, mut rx_die) = tokio::sync::mpsc::unbounded_channel();
    ctrlc::set_handler(move || tx_die.send(()).unwrap())
        .expect("failed to setup graceful shutdown: {}");
    axum::serve(socket, router)
        .with_graceful_shutdown(async move {
            rx_die.recv().await;
        })
        .await
        .expect("server exited improperly: {}");
    trace!("main: cleaning up nicely");
    state.db.close().await;
    Ok(())
}

async fn gen_state() -> MioState {
    trace!("main: creating state");
    let settings = {
        if cfg!(test) {
            SqliteConnectOptions::from_str(":memory:").unwrap()
        } else {
            SqliteConnectOptions::new()
                .filename(DATA_DIR.get().unwrap().join("music.db"))
                .create_if_missing(true)
                .optimize_on_close(true, Some(400))
        }
    };
    let db = SqlitePool::connect_with(settings)
        .await
        .expect("Could not load database: {}");
    trace!("main: migrating database");
    sqlx::migrate!().run(&db).await.unwrap();
    MioState {
        db,
        lock_files: Arc::new(tokio::sync::RwLock::const_new(())),
    }
}

fn gen_public_router(state: MioState) -> Router<()> {
    #[allow(unused)]
    use axum::extract::State;

    async fn ok(State(_): State<MioState>) {}

    Router::new()
        .nest(
            "",
            Router::new()
                // general api stuff, like streaming and querying
                .nest("/api", {
                    #[allow(clippy::let_and_return)]
                    let api = Router::new()
                        .nest("/track", track_manage::routes())
                        .nest("/query", query::routes())
                        .nest("/load", idquery::routes())
                        .nest("/folder", folders::routes());

                    // this is used during testing as a quick method to test for if the auth works
                    if cfg!(test) {
                        api.route("/auth_test", get(ok))
                    } else {
                        api
                    }
                })
                // this is here because it needs the user id from the auth handler
                .route("/user/refresh", patch(user::new_token)),
        )
        // auth handler
        .route_layer(middleware::from_extractor_with_state::<user::Authenticate, _>(state.clone()))
        // user management
        .nest(
            "/user",
            Router::new()
                .route("/login", get(user::login))
                .route("/signup", post(user::signup)),
        )
        // get ver
        .route("/ver", get(get_version))
        // on any panic, dont just leave the client hanging
        .layer(tower_http::catch_panic::CatchPanicLayer::custom(
            |x: Box<dyn std::any::Any + Send>| {
                MioInnerError::Panicked({
                    if let Some(ret) = x.downcast_ref::<&str>() {
                        anyhow::anyhow!("{ret}")
                    } else if let Ok(ret) = x.downcast::<String>() {
                        anyhow::anyhow!("{ret}")
                    } else {
                        anyhow::anyhow!("panic could not be serialized")
                    }
                })
                .into_response()
            },
        ))
        .with_state(state)
}

#[cfg(test)]
pub mod test {
    use super::*;
    use axum::http::{HeaderName, Method};
    use axum_extra::headers::authorization::{Authorization, Credentials};
    use axum_test::{TestRequest, TestServer, TestServerConfig};
    use mio_common::auth;
    use once_cell::sync::Lazy;

    pub static STATE: Lazy<MioState> = Lazy::new(|| futures::executor::block_on(gen_state()));

    // create client
    pub async fn client() -> TestServer {
        // Try to init the logger each time just to make sure stuff is working
        drop(
            env_logger::builder()
                .is_test(true)
                .target(env_logger::Target::Stderr)
                .filter_level(LevelFilter::Debug)
                .try_init(),
        );
        TestServer::new_with_config(
            gen_public_router(STATE.clone()).into_make_service(),
            TestServerConfig {
                ..Default::default()
            },
        )
        .unwrap()
    }

    pub async fn gen_user(client: &TestServer, username: &str) -> auth::JWT {
        let x = client
            .post("/user/signup")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic(username, "password").0.encode(),
            )
            .await;
        if x.status_code() != StatusCode::OK {
            panic!(
                "failed to create user for testing: ({}, {})",
                x.status_code(),
                x.text()
            )
        }
        let jwt = client
            .get("/user/login")
            .add_header(
                HeaderName::from_static("authorization"),
                Authorization::basic(username, "password").0.encode(),
            )
            .await
            .json::<auth::JWT>();
        debug!("token is {jwt:?}");
        jwt
    }

    pub fn jwt_header(
        client: &TestServer,
        method: Method,
        url: &str,
        jwt: &auth::JWT,
    ) -> TestRequest {
        match method {
            Method::GET => client.get(url),
            Method::POST => client.post(url),
            Method::PUT => client.put(url),
            Method::DELETE => client.delete(url),
            Method::PATCH => client.patch(url),
            _ => panic!("method {method:?} is not defined for client creation, plz fix"),
        }
        .add_header(
            HeaderName::from_static("authorization"),
            Authorization::bearer(&jwt.to_string()).unwrap().0.encode(),
        )
    }
}
