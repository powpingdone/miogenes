use actix_web::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use serde_with::base64::{Base64, UrlSafe};
use serde_with::formats::Unpadded;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(track_info);
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrackId(
    #[serde_as(as = "Base64<UrlSafe, Unpadded>")]
    [u8;32]
);

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