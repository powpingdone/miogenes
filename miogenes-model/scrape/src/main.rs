use futures::*;
use governor::*;
use indicatif::*;
use nonzero_ext::*;
use reqwest::header::*;
use reqwest::Client;
use rspotify::clients::BaseClient;
use rspotify::http::*;
use rspotify::model::*;
use rspotify::*;
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;
use std::time::*;
use tokio::fs::*;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc::*;

macro_rules! login {
    () => {{
        let mut x = ClientCredsSpotify::with_config(
            Credentials::from_env().unwrap(),
            Config {
                token_refreshing: true,
                ..Default::default()
            },
        );
        x.request_token().await.unwrap();
        x
    }};
}

macro_rules! wait {
    ($x:expr) => {
        $x.until_ready_with_jitter(Jitter::up_to(Duration::from_secs(1)))
            .await
    };
}

const PLSCRAPE: &str = "Playlist Scrape ::";
const TRSCRAPE: &str = "Track Collect   ::";
const TRDOWNLD: &str = "Track Download  ::";
const PLOUTPUT: &str = "Playlist Record ::";

const PLISTCACHE: &str = "plists.txt";
const TRACKCACHE: &str = "tracks.txt";
const PLAYLISTCSV: &str = "playlists.csv";

const LIMIT: u32 = u32::MAX;
const STEP: u32 = 50;

type GOV = RateLimiter<
    state::direct::NotKeyed,
    state::InMemoryState,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
>;

async fn playlists_scrape(w: Arc<GOV>, bar: ProgressBar, cache: String, tx: Sender<PlaylistId>) {
    bar.set_prefix(PLSCRAPE);

    // load cache
    let mut plists: HashSet<PlaylistId> = HashSet::new();
    bar.set_message("reading cache...");
    let mut contents = Cursor::new(cache);
    let mut buf = String::new();
    while contents.read_line(&mut buf).await.unwrap() != 0 {
        if let Ok(id) = PlaylistId::from_id(buf.as_str()) {
            plists.insert(id);
        }
    }

    // scrape playlists
    let client = login!();
    for term in "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
        .chars()
        .map(|x| x.to_string())
    {
        for inc in (0..LIMIT).step_by(STEP as usize) {
            bar.set_message(format!("scraping \"{term}\": {inc}"));
            wait!(w);
            let req = client
                .search(
                    term.as_str(),
                    &SearchType::Playlist,
                    None,
                    None,
                    Some(STEP),
                    Some(inc),
                )
                .await;
            match req {
                Ok(res) => {
                    if let SearchResult::Playlists(res) = res {
                        for (pos, plist) in res.items.iter().enumerate() {
                            if let Some(name) = plist.owner.display_name.clone() {
                                if name == "Spotify" {
                                    // skip anything made by spotify
                                    continue;
                                }
                            }
                            if plists.insert(plist.id.clone()) {
                                bar.set_message(format!("scraping \"{term}\": {}", pos+inc as usize));
                                tx.send(plist.id.clone()).await.unwrap();
                            }
                        }
                    }
                }
                Err(ret) => {
                    if inc > 300 {
                        break;
                    }
                    panic!("ERR: {ret}");
                }
            }
        }
    }
    bar.set_message("done");
}

async fn playlist_track_scrape(
    w: Arc<GOV>,
    bar: ProgressBar,
    mut rx: Receiver<PlaylistId>,
    tx_track: Sender<(PlaylistId, Vec<(TrackId, String)>)>,
    tx_meta: Sender<(PlaylistId, Vec<TrackId>)>,
) {
    let client = login!();
    bar.set_prefix(TRSCRAPE);
    bar.set_message("waiting...");
    wait!(w);
    'big: while let Some(plist) = rx.recv().await {
        bar.set_message(format!("sending tracks for {plist}"));

        // gather tracks
        let mut tracks_url = vec![];
        let mut tracks = vec![];
        wait!(w);
        let plist_page = client
            .playlist_items_manual(&plist, None, None, Some(1), Some(0))
            .await;
        if let Err(err) = plist_page {
            if let ClientError::Http(resp) = err {
                if let HttpError::StatusCode(resp) = *resp {
                    bar.set_message(format!(
                        "ERR: (Code: {}, Path: {}), waiting...",
                        resp.status(),
                        resp.url()
                    ));
                }
            } else {
                bar.set_message(format!("ERR: {err:?}, waiting..."));
            }
            continue 'big;
        }
        let track_len = plist_page.unwrap().total;
        for amount in (0..(track_len - (track_len % STEP) + STEP)).step_by(STEP as usize) {
            bar.set_message(format!("sending tracks for {plist}: {amount}/{track_len}"));
            wait!(w);
            let plist_page = client
                .playlist_items_manual(&plist, None, None, Some(STEP), Some(amount))
                .await;
            if let Err(err) = plist_page {
                if let ClientError::Http(resp) = err {
                    if let HttpError::StatusCode(resp) = *resp {
                        bar.set_message(format!(
                            "ERR: (Code: {}, Path: {}), waiting...",
                            resp.status(),
                            resp.url()
                        ));
                    }
                } else {
                    bar.set_message(format!("ERR: {err:?}, waiting..."));
                }
                continue 'big;
            }
            for track in plist_page.unwrap().items {
                if let Some(PlayableItem::Track(track)) = track.track {
                    if track.id == None || track.preview_url == None {
                        continue;
                    }
                    let trackid = track.id.unwrap();
                    tracks_url.push((trackid.clone(), track.preview_url.unwrap()));
                    tracks.push(trackid);
                } else {
                    bar.set_message("ERR: Not Track or Does not Exist, waiting...");
                    continue 'big;
                }
            }
        }

        // send tracks
        bar.set_message("waiting...");
        tx_track.send((plist.clone(), tracks_url)).await.unwrap();
        tx_meta.send((plist.clone(), tracks)).await.unwrap();
    }
    bar.set_message("done");
}

async fn track_download_and_rename(
    bar: ProgressBar,
    cache: String,
    mut rx: Receiver<(PlaylistId, Vec<(TrackId, String)>)>,
    tx: Sender<(PlaylistId, Result<(), ()>)>,
) {
    // load cache
    bar.set_prefix(TRDOWNLD);
    bar.set_message("loading cache...");
    let mut track_c: HashSet<TrackId> = HashSet::new();
    let mut contents = Cursor::new(cache);
    let mut buf = String::new();
    while contents.read_line(&mut buf).await.unwrap() != 0 {
        if let Ok(id) = TrackId::from_id(buf.as_str()) {
            track_c.insert(id);
        }
    }

    // download each file
    let client = Client::new();
    let mut cache = OpenOptions::new()
        .append(true)
        .create(true)
        .open(TRACKCACHE)
        .await
        .unwrap();
    bar.set_message("waiting...");
    'big: while let Some((plid, tracks)) = rx.recv().await {
        let full_len = tracks.len();
        for (pos, (trackid, url)) in tracks.iter().enumerate() {
            if track_c.contains(trackid) {
                continue;
            }
            bar.set_message(format!("saving track: ({pos}/{full_len})"));
            match client.get(url).send().await {
                Ok(resp) => {
                    let headers = resp.headers();
                    if None == headers.get(CONTENT_TYPE) {
                        bar.set_message(format!("ERR: {trackid} does not specify Content-Type"));
                        tx.send((plid.clone(), Err(()))).await.unwrap();
                        continue 'big;
                    }
                    let media_type = headers.get(CONTENT_TYPE).unwrap().to_str();
                    if let Err(err) = media_type {
                        bar.set_message(format!(
                            "ERR: Content-Type is not UTF-8 compatible for {trackid}: {err}"
                        ));
                        tx.send((plid.clone(), Err(()))).await.unwrap();
                        continue 'big;
                    }
                    if let Ok(media_type) = media_type.unwrap().parse::<new_mime_guess::Mime>() {
                        if let Some(suffixarr) = new_mime_guess::get_mime_extensions(&media_type) {
                            let suffix = suffixarr[0];
                            let bytes = resp.bytes().await;
                            if let Err(err) = bytes {
                                bar.set_message(format!(
                                    "ERR: Error fetching bytes {trackid}: {err}"
                                ));
                                tx.send((plid.clone(), Err(()))).await.unwrap();
                                continue 'big;
                            }
                            let mut out = File::create(format!("audios/{trackid}.{suffix}"))
                                .await
                                .unwrap();
                            out.write_all(&bytes.unwrap()).await.unwrap();
                            out.sync_all().await.unwrap();
                            track_c.insert(trackid.clone());
                            cache
                                .write_all((trackid.id().to_string() + "\n").as_bytes())
                                .await
                                .unwrap();
                        } else {
                            bar.set_message(format!("ERR: No suffix found for {trackid}"));
                            tx.send((plid.clone(), Err(()))).await.unwrap();
                            continue 'big;
                        }
                    } else {
                        bar.set_message(format!("ERR: No mimetype found for {trackid}"));
                        tx.send((plid.clone(), Err(()))).await.unwrap();
                        continue 'big;
                    }
                }
                Err(err) => {
                    bar.set_message(format!(
                        "ERR: (Code: {}, Path: {}), waiting...",
                        err.status().unwrap(),
                        err.url().unwrap()
                    ));
                    tx.send((plid.clone(), Err(()))).await.unwrap();
                    continue 'big;
                }
            }
        }
        tx.send((plid, Ok(()))).await.unwrap();
        bar.set_message("waiting...");
    }
    bar.set_message("done");
}

async fn write_out_playlist(
    bar: ProgressBar,
    mut rx_pt: Receiver<(PlaylistId, Vec<TrackId>)>,
    mut rx_td: Receiver<(PlaylistId, Result<(), ()>)>,
) {
    let mut csv = OpenOptions::new()
        .append(true)
        .create(true)
        .open(PLAYLISTCSV)
        .await
        .unwrap();
    let mut cache = OpenOptions::new()
        .append(true)
        .create(true)
        .open(PLISTCACHE)
        .await
        .unwrap();
    bar.set_prefix(PLOUTPUT);
    bar.set_message("waiting...");
    while let (Some((plid, tracks)), Some((plid_check, good))) =
        (rx_pt.recv().await, rx_td.recv().await)
    {
        if plid != plid_check {
            panic!("{plid} does not match {plid_check}, SOMEHOW WE DESYNC'D");
        }
        if let Ok(_) = good {
            bar.set_message(format!("writing out {plid}"));
            cache
                .write_all((plid.clone().id().to_string() + "\n").as_bytes())
                .await
                .unwrap();
            csv.write_all(
                format!(
                    "{plid},{}\n",
                    tracks.iter().map(|x| x.id()).collect::<Vec<_>>().join(",")
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        }
        bar.set_message("waiting...");
    }
    bar.set_message("done");
}

#[tokio::main]
async fn main() {
    let gov = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(10_u32))));
    let mp = MultiProgress::new();
    let (pb0, pb1, pb2, pb3) = (
        mp.add(ProgressBar::new_spinner()),
        mp.add(ProgressBar::new_spinner()),
        mp.add(ProgressBar::new_spinner()),
        mp.add(ProgressBar::new_spinner()),
    );
    let spinner_style = ProgressStyle::template(
        ProgressStyle::default_spinner(),
        "{prefix:.bold.dim} {spinner} {wide_msg}",
    )
    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");
    pb0.set_style(spinner_style.clone());
    pb1.set_style(spinner_style.clone());
    pb2.set_style(spinner_style.clone());
    pb3.set_style(spinner_style);

    let (tx_ps, rx_ps) = channel(1000000);
    let (tx_track, rx_track) = channel(1000000);
    let (tx_meta, rx_meta) = channel(1000000);
    let (tx_res, rx_res) = channel(1000000);

    let tasks = [
        tokio::spawn(playlists_scrape(
            gov.clone(),
            pb0,
            read_to_string(PLISTCACHE).await.unwrap_or("".to_string()),
            tx_ps,
        )),
        tokio::spawn(playlist_track_scrape(
            gov.clone(),
            pb1,
            rx_ps,
            tx_track,
            tx_meta,
        )),
        tokio::spawn(track_download_and_rename(
            pb2,
            read_to_string(TRACKCACHE).await.unwrap_or("".to_string()),
            rx_track,
            tx_res,
        )),
        tokio::spawn(write_out_playlist(pb3, rx_meta, rx_res)),
        tokio::task::spawn_blocking(move || mp.join().unwrap()),
    ];
    future::join_all(tasks).await;
}
