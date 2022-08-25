use crate::login_check;
use actix_multipart::Multipart;
use actix_web::{http::header::ContentType, *};
use entity_self::{prelude::*, track_table};
use futures::StreamExt;
use sea_orm::{prelude::*, *};
use serde::Deserialize;
use std::cell::Cell;
use tokio::fs::{remove_file, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
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

    // collect file
    while let Some(field) = payload.next().await {
        if let Err(err) = field {
            return HttpResponse::BadRequest()
                .content_type(ContentType::json())
                .body(
                    crate::MioError {
                        msg: "invalid or corrupt request",
                    }
                    .to_string(),
                );
        }
        let mut field = field.unwrap();

        // TODO: use the user defined upload dir
        // TODO: store the filename for dumping purposes
        // find a unique id for the track
        let uuid: Cell<Uuid> = Cell::new(Uuid::nil());
        let mut file: File;
        loop {
            uuid.set(Uuid::new_v4());
            let fname = format!("./files/{}", uuid.get());
            // check if file is already taken
            let check = OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(fname)
                .await;
            match check {
                Ok(x) => {
                    file = x;
                    break;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::AlreadyExists {
                        continue;
                    }
                    panic!("Failed to open file for writing during an upload: {err}");
                }
            }
        }

        // download the file
        // TODO: filesize limits
        // TODO: maybe don't panic on filesystem errors(?)
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(chunk) => file
                    .write_all(&chunk)
                    .await
                    .expect("Failed to write to file: {}"),
                Err(_) => {
                    // delete failed upload
                    file.flush()
                        .await
                        .expect("Failed to flush uploaded file: {}");
                    drop(file);
                    remove_file(format!("./files/{}", uuid.get()))
                        .await
                        .expect("Failed to delete uploaded file: {}");

                    return HttpResponse::BadRequest()
                        .content_type(ContentType::json())
                        .body(
                            crate::MioError {
                                msg: "invalid or corrupt request or chunk",
                            }
                            .to_string(),
                        );
                }
            }
        }

        // set off tasks to process files
        // tx.send((uuid, userid)).await.unwrap()
    }
    HttpResponse::Ok().finish()
}
