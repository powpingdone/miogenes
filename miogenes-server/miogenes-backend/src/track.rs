use crate::login_check;
use actix_web::{http::header::ContentType, *};
use actix_multipart::Multipart;
use entity_self::{prelude::*, track_table};
use sea_orm::{prelude::*, *};
use serde::Deserialize;
use uuid::Uuid;

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(track_info).service(track_upload);
}

#[derive(Debug, Deserialize)]
struct TInfoQuery {
    #[serde(rename = "tr")]
    trackid: Uuid,
}

#[get("/ti")]
async fn track_info(
    db: web::Data<DatabaseConnection>,
    key: web::Query<crate::User>,
    track: web::Query<TInfoQuery>,
) -> impl Responder {
    let (db, userid) = login_check!(db, key);
    let track = track.into_inner();

    // contact the database and query it
    let resp = TrackTable::find_by_id(track.trackid)
        .filter(Condition::all().add(track_table::Column::Owner.eq(userid)))
        .one(db.as_ref())
        .await;

    match resp {
        // database fails to talk
        Err(err) => HttpResponse::InternalServerError()
            .content_type(ContentType::json())
            .body(
                crate::MioError {
                    msg: format!("database error for {}: {err}", track.trackid).as_str(),
                }
                .to_string(),
            ),
        Ok(resp) => match resp {
            // track doesn't exist
            None => HttpResponse::NotFound()
                .content_type(ContentType::json())
                .body(
                    crate::MioError {
                        msg: format!("no track found for {}", track.trackid).as_str(),
                    }
                    .to_string(),
                ),
            Some(content) => match serde_json::to_string(&content) {
                Ok(json) => HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(json),
                // somehow, serialization failed
                Err(err) => HttpResponse::InternalServerError()
                    .content_type(ContentType::json())
                    .body(
                        crate::MioError {
                            msg: format!(
                                "internal serialization error for {}: {err}",
                                track.trackid
                            )
                            .as_str(),
                        }
                        .to_string(),
                    ),
            },
        },
    }
}

#[put("/tu")]
async fn track_upload(
    db: web::Data<DatabaseConnection>,
    key: web::Query<crate::User>,
    mut payload: Multipart,
) -> impl Responder {
    let (db, userid) = login_check!(db, key);
    HttpResponse::Ok().finish()
}
