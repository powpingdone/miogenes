use actix_web::http::header::ContentType;
use actix_web::*;
use entity_self::prelude::*;
use entity_self::*;
use sea_orm::prelude::Uuid;
use sea_orm::*;
use tokio::task::JoinHandle;

#[derive(serde::Serialize)]
struct HeartBeat {
    album_art: Vec<(Uuid, u64)>,
    album: Vec<(Uuid, u64)>,
    artist: Vec<(Uuid, u64)>,
    track: Vec<(Uuid, u64)>,
}

#[derive(serde::Deserialize)]
struct HBQuery {
    #[serde(rename = "ts")]
    timestamp: Option<u64>,
}

macro_rules! HB_Task {
    ($up: tt, $down: tt, $db: expr, $t: expr, $u: expr) => {{
        let db = $db.clone();
        tokio::spawn(async move {
            $up::find()
                .filter(
                    Condition::all()
                        .add($down::Column::Ts.gte($t))
                        .add($down::Column::Owner.eq($u)),
                )
                .order_by_asc($down::Column::Ts)
                .limit(250)
                .all(db.as_ref())
                .await
        })
    }};
}

macro_rules! HB_Unwrap {
    ($info: expr) => {{
        let x = $info.await;
        match x {
            Ok(x) => match x {
                Ok(x) => x
                    .iter()
                    .map(|obj| (obj.id, obj.ts as u64))
                    .collect::<Vec<_>>(),
                Err(_) => {
                    return HttpResponse::InternalServerError()
                        .content_type(ContentType::json())
                        .body(
                            crate::MioError {
                                msg: "$info task failed to query database",
                            }
                            .to_string(),
                        )
                }
            },
            Err(_) => {
                return HttpResponse::InternalServerError()
                    .content_type(ContentType::json())
                    .body(
                        crate::MioError {
                            msg: "$info task failed to execute",
                        }
                        .to_string(),
                    )
            }
        }
    }};
}

/*  The heartbeat function.

    This takes an optional timestamp (from the unix epoch) and produces 250
    Uuids from each table and their respective timestamps. This function's
    purpose is to maintain consistency across clients when a simultanious
    upload/download is happening. In the future, this may also return
    playlists and other user generated content.
*/
#[get("/hb")]
async fn heartbeat(
    db: web::Data<DatabaseConnection>,
    key: web::Query<crate::User>,
    tstamp: web::Query<HBQuery>,
) -> impl Responder {
    // generic setup
    let db = db.into_inner();
    let key = key.into_inner();
    let userid = key.check(&db).await;
    if let Err(ret) = userid {
        return ret;
    }
    let userid = userid.unwrap();
    let tstamp = tstamp.into_inner().timestamp.unwrap_or(0);

    // query database for updates
    struct HBResp {
        album_art: JoinHandle<Result<Vec<album_art_table::Model>, DbErr>>,
        album: JoinHandle<Result<Vec<album_table::Model>, DbErr>>,
        artist: JoinHandle<Result<Vec<artist_table::Model>, DbErr>>,
        track: JoinHandle<Result<Vec<track_table::Model>, DbErr>>,
    }

    let search_ret = HBResp {
        album_art: HB_Task!(AlbumArtTable, album_art_table, db, tstamp, userid),
        album: HB_Task!(AlbumTable, album_table, db, tstamp, userid),
        artist: HB_Task!(ArtistTable, artist_table, db, tstamp, userid),
        track: HB_Task!(TrackTable, track_table, db, tstamp, userid),
    };

    // put into hb return struct
    let ret = HeartBeat {
        album_art: HB_Unwrap!(search_ret.album_art),
        album: HB_Unwrap!(search_ret.album),
        artist: HB_Unwrap!(search_ret.artist),
        track: HB_Unwrap!(search_ret.track),
    };

    HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&ret).unwrap())
}
