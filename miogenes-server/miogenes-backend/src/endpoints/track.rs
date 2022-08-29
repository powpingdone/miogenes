use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::*;
use entity_self::{prelude::*, track_table};
use futures::StreamExt;
use sea_orm::{prelude::*, *};
use serde::Deserialize;
use std::cell::Cell;
use std::sync::Arc;
use tokio::fs::{remove_file, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub fn routes() -> Router {
    Router::new().route("/ti", get(track_info))
}

#[derive(Debug, Deserialize)]
struct TInfoQuery {
    #[serde(rename = "tr")]
    trackid: Uuid,
}

async fn track_info(
    state: Extension<Arc<crate::MioState>>,
    key: Query<crate::User>,
    track: Query<TInfoQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let userid = crate::login_check(state.db.clone(), key.0).await?;

    // contact the database and query it
    let resp = TrackTable::find_by_id(track.trackid)
        .filter(Condition::all().add(track_table::Column::Owner.eq(userid)))
        .one(state.db.as_ref())
        .await;

    match resp {
        // database fails to talk
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::MioError {
                msg: format!("database error for {}: {err}", track.trackid),
            }),
        )),
        Ok(resp) => match resp {
            // track doesn't exist
            None => Err((
                StatusCode::NOT_FOUND,
                Json(crate::MioError {
                    msg: format!("no track found for {}", track.trackid),
                }),
            )),
            Some(content) => match serde_json::to_string(&content) {
                Ok(json) => Ok((StatusCode::OK, Json(json))),
                // somehow, serialization failed
                Err(err) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(crate::MioError {
                        msg: format!("internal serialization error for {}: {err}", track.trackid),
                    }),
                )),
            },
        },
    }
}

/* #[put("/tu")]
async fn track_upload(
    State(state): State<crate::MioState>,
    key: Query<crate::User>,
) -> impl Responder {
    let userid = login_check!(db, key);
    let tx = state;

    // collect file
    while let Some(field) = payload.next().await {
        if field.is_err() {
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

        // TODO: store the filename for dumping purposes
        // find a unique id for the track
        let uuid: Cell<Uuid> = Cell::new(Uuid::nil());
        let mut file: File;
        loop {
            uuid.set(Uuid::new_v4());
            let fname = format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid.get());
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

        // get original filename
        let orig_filename = field
            .content_disposition()
            .get_filename()
            .map_or_else(|| uuid.get().to_string(), |ret| ret.to_string());

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
                    remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid.get()))
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
        tx.send((uuid.get(), userid, orig_filename)).unwrap();
    }
    HttpResponse::Ok().finish()
}
 */
