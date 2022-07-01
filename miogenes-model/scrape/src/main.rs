use futures::*;
use governor::*;
use indicatif::*;
use nonzero_ext::*;
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

type Gov = RateLimiter<
    state::direct::NotKeyed,
    state::InMemoryState,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
>;

async fn playlists_scrape(w: Arc<Gov>, bar: ProgressBar, cache: String, tx: Sender<PlaylistId>) {
    bar.set_prefix(PLSCRAPE);

    // load cache
    let mut plists: HashSet<PlaylistId> = HashSet::new();
    bar.set_message("reading cache...");
    let mut contents = Cursor::new(cache);
    let mut buf = String::new();
    while contents.read_line(&mut buf).await.unwrap() != 0 {
        match PlaylistId::from_id_or_uri(buf.trim()) {
            Ok(id) => {
                plists.insert(id);
                ()
            }
            Err(err) => bar.set_message(format!("Err: {buf} {err}")),
        }
        buf.clear();
    }
    bar.set_message(format!("loaded {} playlists from cache", plists.len()));
    drop(contents);
    drop(buf);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // scrape playlists
    let client = login!();
    for term in "0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz"
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
                                bar.set_message(format!(
                                    "scraping \"{term}\": {}",
                                    pos + inc as usize
                                ));
                                tx.send(plist.id.clone()).await.unwrap();
                            }
                        }
                    }
                }
                Err(ret) => {
                    if let ClientError::Http(_) = ret {
                        if inc > 300 {
                            break;
                        }
                        bar.set_message(format!("ERR: {ret}"));
                        return;
                    }
                    bar.set_message(format!("ERR: {ret}"));
                }
            }
        }
    }
    bar.set_message("done");
}

async fn playlist_track_scrape(
    w: Arc<Gov>,
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

        // get all tracks
        bar.set_message(format!("gathering tracks for {plist}"));
        let track_len = plist_page.unwrap().total;
        let stream = (0..track_len - (track_len % STEP) + STEP)
            .step_by(STEP as usize)
            .map(|amount| {
                let client = &client;
                let w = w.clone();
                let plist = &plist;
                async move {
                    wait!(w);
                    (
                        amount,
                        client
                            .playlist_items_manual(plist, None, None, Some(STEP), Some(amount))
                            .await,
                    )
                }
            });
        let streams = stream.len();
        let mut pages = stream::iter(stream)
            .buffer_unordered(streams)
            .collect::<Vec<_>>()
            .await;
        // since i'm forced to do buffer_unordered, sort it afterwards
        pages.sort_unstable_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // create the vec of tracks
        for page in pages.into_iter() {
            bar.set_message(format!("sending tracks for {plist}"));
            let plist_page = page.1;
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
    let mut buf = String::with_capacity(50);
    while contents.read_line(&mut buf).await.unwrap() != 0 {
        match TrackId::from_id_or_uri(buf.trim()) {
            Ok(id) => {
                track_c.insert(id);
                ()
            }
            Err(err) => bar.set_message(format!("Err: {buf} {err}")),
        }
        buf.clear();
    }
    bar.set_message(format!("loaded {} tracks from cache", track_c.len()));
    drop(contents);
    drop(buf);
    tokio::time::sleep(Duration::from_millis(500)).await;

    // download each file
    let client = Client::new();
    let mut cache = OpenOptions::new()
        .append(true)
        .create(true)
        .open(TRACKCACHE)
        .await
        .unwrap();
    bar.set_message("waiting...");
    while let Some((plid, tracks)) = rx.recv().await {
        // filter out tracks already done
        bar.set_message("filtering tracks input...");
        let true_len = tracks.len();
        let tracks = tracks
            .clone()
            .into_iter()
            .filter_map(|item| {
                if track_c.contains(&item.0) {
                    None
                } else {
                    Some(item)
                }
            })
            .collect::<Vec<_>>();
        let full_len = tracks.len();

        // stream to download files
        let mut streams = stream::iter(tracks.into_iter().map(|(trackid, url)| {
            let client = &client;
            async move {
                let resp = client.get(url).send().await?;
                let bytes = resp.bytes().await?;
                let mut out = File::create(format!("audios/{trackid}.mp3")).await?;
                out.write_all(&bytes).await?;
                out.sync_all().await?;
                Ok::<_, anyhow::Error>(trackid.to_owned())
            }
        }))
        .buffer_unordered(30)
        .enumerate();

        // update progress bars/err msgs
        let mut good = true;
        let mut msg = "downloading...".to_string();
        while let Some((pos, ret)) = streams.next().await {
            match ret {
                Ok(trackid) => {
                    track_c.insert(trackid.clone());
                    cache
                        .write_all((trackid.uri().to_string() + "\n").as_bytes())
                        .await
                        .unwrap();
                }
                Err(err) => {
                    if good {
                        good = false;
                        tx.send((plid.clone(), Err(()))).await.unwrap();
                    }
                    msg = format!("{err}");
                }
            }
            bar.set_message(format!(
                "({}/{} out of possible {}) {}",
                pos, full_len, true_len, msg
            ));
        }
        if good {
            tx.send((plid, Ok(()))).await.unwrap();
        }
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
    let mut cached: usize = 0;
    let mut total: usize = 0;
    while let (Some((plid, tracks)), Some((plid_check, good))) =
        (rx_pt.recv().await, rx_td.recv().await)
    {
        if plid != plid_check {
            bar.set_message(format!(
                "{plid} does not match {plid_check}, SOMEHOW WE DESYNC'D"
            ));
            return;
        }
        if good.is_ok() {
            bar.set_message(format!("caching {plid}"));
            cache
                .write_all((plid.clone().uri().to_string() + "\n").as_bytes())
                .await
                .unwrap();
            cached += 1;
            if tracks.len() > 8 {
                bar.set_message(format!("writing out {plid}"));
                csv.write_all(
                    format!(
                        "{plid},{}\n",
                        tracks.iter().map(|x| x.id()).collect::<Vec<_>>().join(",")
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
                total += 1;
            }
        }
        bar.set_message(format!(
            "{total} out of possible {cached} playlists written out..."
        ));
    }
    bar.set_message("done");
}

#[tokio::main]
async fn main() {
    let gov = Arc::new(RateLimiter::direct(Quota::per_second(nonzero!(8_u32))));
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
    .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
    .on_finish(ProgressFinish::AndLeave);
    pb0.set_style(spinner_style.clone());
    pb1.set_style(spinner_style.clone());
    pb2.set_style(spinner_style.clone());
    pb3.set_style(spinner_style);
    pb0.enable_steady_tick(2000);
    pb1.enable_steady_tick(750);
    pb2.enable_steady_tick(450);
    pb3.enable_steady_tick(5000);

    let (tx_ps, rx_ps) = channel(300);
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
