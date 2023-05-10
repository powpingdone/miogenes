

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
