use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::*;
use log::*;
use once_cell::sync::OnceCell;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;
use std::sync::Arc;
use subtasks::secret::SecretHolder;
use utoipa::OpenApi;

mod db;
mod endpoints;
mod error;
mod subtasks;
mod user;

pub(crate) use crate::error::*;
use endpoints::*;

// TODO: use the user supplied dir
static DATA_DIR: OnceCell<&str> = OnceCell::with_value({
    cfg_if::cfg_if! {
        if #[cfg(test)] {
            "test_files"
        }
        else {
            "files"
        }
    }
});

#[derive(Clone)]
pub struct MioState {
    db: SqlitePool,
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

#[utoipa::path(
    get,
    path = "/ver",
    responses(
        (status = 200, response = mio_common::Vers)
    ),
)]
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

pub fn gen_openapi() -> String {
    #[derive(OpenApi)]
    #[openapi(paths(get_version), components(responses(mio_common::Vers)))]
    struct Api;
    Api::openapi().to_pretty_json().unwrap()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO: tracing
    env_logger::builder()
        .filter_level(LevelFilter::Trace)
        .try_init()?;
    gstreamer::init()?;

    // generate openapi stuff for autogen of libraries
    #[cfg(generate_api_dot_json)]
    {
        return std::fs::write("./api.json", gen_openapi()).map_err(anyhow::Error::new);
    }

    #[cfg(not(generate_api_dot_json))]
    {
        // create the main passing state
        let state = gen_state().await;

        // setup the router
        trace!("main: building router");
        let router = gen_router(state.clone());

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
}

async fn gen_state() -> MioState {
    trace!("main: creating state");
    let settings = {
        cfg_if::cfg_if! {
            if #[cfg(test)] {
                SqliteConnectOptions::from_str(":memory:").unwrap()
            }
            else {
                SqliteConnectOptions::from_str("sqlite://files/music.db").unwrap().create_if_missing(true)
            }
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
        secret: SecretHolder::new().await,
    }
}

fn gen_router(state: MioState) -> Router<()> {
    #[allow(unused)]
    use axum::extract::State;

    #[cfg(test)]
    async fn ok(State(_): State<MioState>) {}

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
            cfg_if::cfg_if! {
                if #[cfg(test)] {
                    api.route("/auth_test", get(ok))
                }
                else {
                    api
                }
            }
        })
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
        // always log what request is coming through
        .layer(axum::middleware::from_fn(log_req))
        .with_state(state)
}

// small logging function that logs the method (eg, GET) and endpoint uri
async fn log_req<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
    info!("{} {}", req.method(), req.uri());
    next.run(req).await
}

#[cfg(test)]
pub mod test {
    use super::*;
    use axum::{
        headers::authorization::{Authorization, Credentials},
        http::{HeaderName, Method},
    };
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
            gen_router(STATE.clone()).into_make_service(),
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

