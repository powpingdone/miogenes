use actix_web::middleware::Logger;
use actix_web::*;
use serde::Serialize;

mod heartbeat;
mod track;

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

    let db = sea_orm::Database::connect("postgres://user:password@127.0.0.1:5432/db")
        .await
        .expect("failed to connect to db: {}");
    Migrator::up(&db, None).await?;

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .service(version)
            .service(heartbeat::heartbeat)
            .service(web::scope("/track").configure(track::routes))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;
    Ok(())
}
