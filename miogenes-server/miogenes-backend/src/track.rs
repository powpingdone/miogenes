use actix_web::*;
use serde::{Deserialize, Serialize};

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(track_info).service(track_upload);
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackId(u64);

#[derive(Debug, Deserialize)]
struct TInfoQuery {
    #[serde(rename = "tr")]
    _trackid: TrackId,
}

#[get("/ti")]
async fn track_info(_track: web::Query<TInfoQuery>) -> impl Responder {
    web::Json({})
}

#[put("/tu")]
async fn track_upload() -> impl Responder {
    web::Json({})
}
