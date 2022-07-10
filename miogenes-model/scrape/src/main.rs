use futures::*;
use governor::*;
use indicatif::*;
use nonzero_ext::*;
use rspotify::clients::BaseClient;
use rspotify::http::*;
use rspotify::model::*;
use rspotify::*;
use std::collections::HashSet;
use std::io::Cursor;
use std::sync::Arc;
use std::time::*;
use tokio::fs::*;
use tokio::io::AsyncWriteExt;
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
const PLFILTER: &str = "Playlist Filter ::";
const TRSCRAPE: &str = "Track Collect   ::";
const PLOUTPUT: &str = "Playlist Record ::";

const PLISTCACHE: &str = "plists.txt";
const PLAYLISTCSV: &str = "playlists.csv";

const LIMIT: u32 = u32::MAX;
const STEP: u32 = 50;

type Gov = RateLimiter<
    state::direct::NotKeyed,
    state::InMemoryState,
    clock::DefaultClock,
    middleware::NoOpMiddleware,
>;

async fn playlists_scrape(w: Arc<Gov>, bar: ProgressBar, tx: Sender<Page<SimplifiedPlaylist>>) {
    bar.set_prefix(PLSCRAPE);

    // scrape playlists
    const TERMS: &str =
        "0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz";
    const TERMSSPC: &str =
        " 0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz";
    let client = login!();
    for term0 in TERMS.chars().map(|x| x.to_string()) {
        for term1 in TERMSSPC.chars().map(|x| x.to_string()) {
            for inc in (0..LIMIT).step_by(STEP as usize) {
                bar.set_message(format!("scraping \"{term0}{term1}\": {inc}"));
                wait!(w);
                let req = client
                    .search(
                        format!("{term0}{term1}").as_str(),
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
                            tx.send(res).await.unwrap();
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
    }
    bar.set_message("done");
}

fn playlist_filter(
    bar: ProgressBar,
    cache: String,
    mut rx: Receiver<Page<SimplifiedPlaylist>>,
    tx: Sender<PlaylistId>,
) {
    use std::io::BufRead;
    use std::thread::sleep;
    bar.set_prefix(PLFILTER);

    let mut plists: HashSet<PlaylistId> = HashSet::new();
    bar.set_message("reading cache...");
    let mut contents = Cursor::new(cache);
    let mut buf = String::new();
    while BufRead::read_line(&mut contents, &mut buf).unwrap() != 0 {
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
    sleep(Duration::from_millis(500));

    let mut procd: usize = 0;
    while let Some(page) = rx.blocking_recv() {
        for plist in page.items.iter() {
            if let Some(name) = plist.owner.display_name.clone() {
                if name == "Spotify" {
                    // skip anything made by spotify
                    continue;
                }
            }
            if plists.insert(plist.id.clone()) {
                tx.blocking_send(plist.id.clone()).unwrap();
            }
            procd += 1;
            bar.set_message(format!("filtered: {procd}"));
        }
    }

    bar.set_message("done");
}

async fn playlist_track_scrape(
    w: Arc<Gov>,
    bar: ProgressBar,
    mut rx: Receiver<PlaylistId>,
    tx_meta: Sender<(PlaylistId, Vec<TrackId>)>,
) {
    let client = login!();
    bar.set_prefix(TRSCRAPE);
    bar.set_message("waiting...");
    wait!(w);
    'big: while let Some(plist) = rx.recv().await {
        bar.set_message(format!("sending tracks for {plist}"));

        // gather tracks
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
                    tracks.push(trackid);
                }
            }
        }

        // send tracks
        bar.set_message("waiting...");
        tx_meta.send((plist.clone(), tracks)).await.unwrap();
    }
    bar.set_message("done");
}

async fn write_out_playlist(
    bar: ProgressBar,
    mut rx_pt: Receiver<(PlaylistId, Vec<TrackId>)>,
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
    while let Some((plid, tracks)) =
        rx_pt.recv().await
    {
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
    pb1.enable_steady_tick(1000);
    pb2.enable_steady_tick(750);
    pb3.enable_steady_tick(5000);

    let (tx_ps, rx_ps) = channel(60);
    let (tx_pfil, rx_pfil) = channel(300);
    let (tx_meta, rx_meta) = channel(1000000);

    let tasks = [
        tokio::spawn(playlists_scrape(gov.clone(), pb0, tx_ps)),
        tokio::task::spawn_blocking(|| {
            playlist_filter(
                pb1,
                std::fs::read_to_string(PLISTCACHE).unwrap_or("".to_string()),
                rx_ps,
                tx_pfil,
            )
        }),
        tokio::spawn(playlist_track_scrape(
            gov.clone(),
            pb2,
            rx_pfil,
            tx_meta,
        )),
        tokio::spawn(write_out_playlist(pb3, rx_meta)),
        tokio::task::spawn_blocking(move || mp.join().unwrap()),
    ];
    future::join_all(tasks).await;
}
