use tracing::Level;

mod ssr;

#[derive(Clone)]
struct MioState {}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let state = make_state();
    let router = make_router(state);
    axum_server::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 8080)))
        .serve(router.into_make_service())
        .await
        .unwrap();
}

fn make_state() -> MioState {
    MioState {}
}

fn make_router(state: MioState) -> axum::Router<()> {
    use axum::extract::*;
    use axum::routing::*;
    use tower_http::trace::*;

    axum::Router::new()
        .route("", get(|_: State<MioState>| async {}))
        // tracing (logging)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}
