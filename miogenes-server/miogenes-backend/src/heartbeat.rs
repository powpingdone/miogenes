use actix_web::*;
use std::time::*;

#[derive(serde::Serialize)]
struct HeartBeat {
    timestamp: u64,
}

#[get("/hb")]
async fn heartbeat() -> impl Responder {
    let ret = HeartBeat {
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("The time has gone backwards! This will be handled in the future.").as_secs(),
    };
    web::Json(ret)
}
