use actix_web::*;
use sea_orm::DatabaseConnection;
use std::time::*;

#[derive(serde::Serialize)]
struct HeartBeat {
    timestamp: u64,
}

#[derive(serde::Deserialize)]
struct HBQuery {
    #[serde(rename = "ts")]
    timestamp: Option<u64>,
}

#[get("/hb")]
async fn heartbeat(
    db: web::Data<DatabaseConnection>,
    tstamp: web::Query<HBQuery>,
    key: web::Query<crate::User>,
) -> impl Responder {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time ran backwards! this will (possibly) be handled in the future")
        .as_secs();
    let key = key.into_inner();
    let db = db.into_inner();
    let tstamp = tstamp.into_inner().timestamp;

    web::Json(HeartBeat { timestamp: ts })
}
