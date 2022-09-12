use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::*;
use entity_self::prelude::*;
use entity_self::*;
use log::*;
use sea_orm::prelude::Uuid;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Serialize)]
struct HeartBeat {
    album_art: Vec<(Uuid, u64)>,
    album: Vec<(Uuid, u64)>,
    artist: Vec<(Uuid, u64)>,
    track: Vec<(Uuid, u64)>,
}

#[derive(Deserialize)]
pub struct HBQuery {
    #[serde(rename = "ts")]
    timestamp: Option<u64>,
}

macro_rules! HB_Task {
    ($up: tt, $down: tt, $db: expr, $t: expr, $u: expr) => {{
        let db = $db.clone();
        tokio::spawn(async move {
            trace!("/hb spinning task $up for $u");
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
                Ok(x) => {
                    let ret = x
                        .iter()
                        .map(|obj| (obj.id, obj.ts as u64))
                        .collect::<Vec<_>>();
                    trace!("/hb $info collected {} uuids", ret.len());
                    ret
                }
                Err(err) => {
                    error!("/hb $info task failed to query db: {err}");
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(crate::MioError {
                            msg: "$info task failed to query database".to_owned(),
                        }),
                    ));
                }
            },
            Err(err) => {
                error!("/hb $info task failed to execute: {err}");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(crate::MioError {
                        msg: "$info task failed to execute".to_owned(),
                    }),
                ));
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
pub async fn heartbeat(
    state: Extension<Arc<crate::MioState>>,
    key: Query<crate::User>,
    tstamp: Query<HBQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // generic setup
    let userid = crate::login_check(state.db.clone(), key.0).await?;
    let tstamp = tstamp.timestamp.unwrap_or(0);

    // query database for updates
    struct HBResp {
        album_art: JoinHandle<Result<Vec<album_art_table::Model>, DbErr>>,
        album: JoinHandle<Result<Vec<album_table::Model>, DbErr>>,
        artist: JoinHandle<Result<Vec<artist_table::Model>, DbErr>>,
        track: JoinHandle<Result<Vec<track_table::Model>, DbErr>>,
    }

    let search_ret = HBResp {
        album_art: HB_Task!(AlbumArtTable, album_art_table, state.db, tstamp, userid),
        album: HB_Task!(AlbumTable, album_table, state.db, tstamp, userid),
        artist: HB_Task!(ArtistTable, artist_table, state.db, tstamp, userid),
        track: HB_Task!(TrackTable, track_table, state.db, tstamp, userid),
    };

    // put into hb return struct
    let ret = HeartBeat {
        album_art: HB_Unwrap!(search_ret.album_art),
        album: HB_Unwrap!(search_ret.album),
        artist: HB_Unwrap!(search_ret.artist),
        track: HB_Unwrap!(search_ret.track),
    };

    Ok((StatusCode::OK, Json(ret)))
}
