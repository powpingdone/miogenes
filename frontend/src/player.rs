use std::{io::Cursor, pin::Pin};

use image::RgbaImage;
use mio_glue::player::{CurrentlyDecoding, Player};
use slint::Rgba8Pixel;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::*;

pub fn start_player_hold_thread(
    client: Arc<RwLock<MioClientState>>,
    rt: &tokio::runtime::Runtime,
) -> (
    crossbeam::channel::Sender<DecoderMsg>,
    tokio::sync::watch::Receiver<CurrentlyDecoding>,
) {
    let (tx, rx) = oneshot::channel();
    rt.spawn_blocking(|| player_thread(client, tx));
    rx.blocking_recv().unwrap()
}

fn player_thread(
    client: Arc<RwLock<MioClientState>>,
    ret: oneshot::Sender<(
        crossbeam::channel::Sender<DecoderMsg>,
        tokio::sync::watch::Receiver<CurrentlyDecoding>,
    )>,
) {
    let player = Player::new(client).unwrap();
    ret.send((player.tx.clone(), player.rx.clone())).unwrap();
    loop {}
}

impl MioFrontendWeak {
    pub fn start_player_poll_task(&self) -> tokio::task::JoinHandle<()> {
        self.w_rt().unwrap().spawn(self.clone().poll_task())
    }

    async fn poll_task(self) {
        let mut state = PollTaskState::new();
        while let Ok(mut rx) = self.w_player_rx() {
            rx.changed().await.unwrap();
            let curr_decoding = rx.borrow_and_update().clone();
            let (is_new_state, is_track_diff) = state.set_new_decoding(&curr_decoding);
            // invalidate UI metadata if different track
            if is_track_diff {
                self.app
                    .upgrade_in_event_loop(|app| {
                        app.global::<crate::PlayerCB>().set_loaded(false);
                    })
                    .unwrap();
                // spawn task to fetch new metadata
                let rt = self.w_rt().unwrap();
                rt.spawn(state.new_fetch_task(curr_decoding.curr, self.state.clone()));
            }

            // set fetched mdata
            if is_new_state {
                if state.is_all_mdata_ready()
                    && !self.w_app().unwrap().global::<PlayerCB>().get_loaded()
                {
                    let title = state.title.as_ref().unwrap().into();
                    let album = state
                        .album
                        .as_ref()
                        .map(String::as_str)
                        .unwrap_or("?")
                        .into();
                    let artist = state
                        .artist
                        .as_ref()
                        .map(String::as_str)
                        .unwrap_or("?")
                        .into();
                    let img = image_get(state.cover_art.as_ref().map(Vec::as_slice));
                    self.app
                        .upgrade_in_event_loop({
                            move |app| {
                                let cb = app.global::<PlayerCB>();
                                cb.set_loaded(true);
                                cb.set_title(title);
                                cb.set_album(album);
                                cb.set_artist(artist);
                                cb.set_cover_art(
                                    // img is unwrapped in loop due to slint::Image not being Send
                                    //
                                    // let's hope that the memcpy is fast enough...!
                                    img.map(|img| {
                                        slint::Image::from_rgba8(slint::SharedPixelBuffer::<
                                            Rgba8Pixel,
                                        >::clone_from_slice(
                                            img.as_raw(),
                                            img.width(),
                                            img.height(),
                                        ))
                                    })
                                    .unwrap_or_else(|_| {
                                        slint::Image::from_rgba8(slint::SharedPixelBuffer::new(
                                            0, 0,
                                        ))
                                    }),
                                );
                            }
                        })
                        .unwrap();
                }
                // set currently decoding metadata
                self.app
                    .upgrade_in_event_loop(move |app| {
                        let cb = app.global::<PlayerCB>();
                        cb.set_length(curr_decoding.len.as_secs_f32());
                        cb.set_playback_pos(curr_decoding.at.as_secs_f32());
                    })
                    .unwrap();
            }
        }
    }
}

fn image_get(buf: Option<&[u8]>) -> anyhow::Result<RgbaImage> {
    Ok(
        image::io::Reader::new(Cursor::new(buf.ok_or(anyhow::anyhow!("no image"))?))
            .with_guessed_format()?
            .decode()?
            .into_rgba8(),
    )
}

struct PollTaskState {
    prev_state: Option<CurrentlyDecoding>,
    pub title: Option<String>,
    title_rx: oneshot::Receiver<String>,
    pub album: Option<String>,
    album_rx: oneshot::Receiver<Option<String>>,
    pub artist: Option<String>,
    artist_rx: oneshot::Receiver<Option<String>>,
    pub cover_art: Option<Vec<u8>>,
    cover_art_rx: oneshot::Receiver<Option<Vec<u8>>>,
}

impl PollTaskState {
    pub fn new() -> Self {
        Self {
            // create rx in dead state
            album_rx: oneshot::channel().1,
            artist_rx: oneshot::channel().1,
            cover_art_rx: oneshot::channel().1,
            title_rx: oneshot::channel().1,

            prev_state: None,
            album: None,
            artist: None,
            cover_art: None,
            title: None,
        }
    }

    pub fn set_new_decoding(&mut self, new_state: &CurrentlyDecoding) -> (bool, bool) {
        let is_new = !self.prev_state.as_ref().is_some_and(|x| *x == *new_state);
        let diff_track = !self
            .prev_state
            .as_ref()
            .is_some_and(|x| x.curr == new_state.curr);
        self.prev_state = Some(new_state.clone());

        (is_new, diff_track)
    }

    pub fn new_fetch_task(
        &mut self,
        id: Uuid,
        state: StdWeak<RwLock<MioClientState>>,
    ) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        // future for recv mdata
        let (title_tx, title_rx) = oneshot::channel();
        let (album_tx, album_rx) = oneshot::channel();
        let (artist_tx, artist_rx) = oneshot::channel();
        let (cover_art_tx, cover_art_rx) = oneshot::channel();
        let fut = Box::pin(fetch_track_mdata(
            state,
            id,
            title_tx,
            album_tx,
            artist_tx,
            cover_art_tx,
        ));

        // set state
        self.title.take();
        self.album.take();
        self.artist.take();
        self.cover_art.take();
        self.title_rx = title_rx;
        self.album_rx = album_rx;
        self.artist_rx = artist_rx;
        self.cover_art_rx = cover_art_rx;

        fut
    }

    pub fn is_all_mdata_ready(&mut self) -> bool {
        macro_rules! check_and_set {
            ($field:ident, $rx:ident) => {
                match self.$rx.try_recv() {
                    Ok(x) => {
                        self.$field = x.into();
                        true
                    }
                    Err(oneshot::error::TryRecvError::Empty) => false,
                    Err(oneshot::error::TryRecvError::Closed) => true,
                }
            };
        }

        check_and_set!(title, title_rx)
            && check_and_set!(artist, artist_rx)
            && check_and_set!(album, album_rx)
            && check_and_set!(cover_art, cover_art_rx)
    }
}

// TODO: error reporting
async fn fetch_track_mdata(
    state: StdWeak<RwLock<MioClientState>>,
    track: Uuid,
    title: oneshot::Sender<String>,
    album: oneshot::Sender<Option<String>>,
    artist: oneshot::Sender<Option<String>>,
    cover_art: oneshot::Sender<Option<Vec<u8>>>,
) {
    let h_state = state.upgrade().unwrap();
    let state = h_state.read().await;
    let track_whole = state.get_track_data(track).await.unwrap();
    tokio::spawn({
        let h_state = h_state.clone();
        let id = track_whole.album;
        async move {
            if let Some(id) = id {
                let state = h_state.read().await;
                let _ = album.send(Some(state.get_album_data(id).await.unwrap().title));
            } else {
                let _ = album.send(None);
            }
        }
    });
    tokio::spawn({
        let h_state = h_state.clone();
        let id = track_whole.artist;
        async move {
            if let Some(id) = id {
                let state = h_state.read().await;
                let _ = artist.send(Some(state.get_artist_data(id).await.unwrap().name));
            } else {
                let _ = artist.send(None);
            }
        }
    });
    tokio::spawn({
        let h_state = h_state.clone();
        let id = track_whole.cover_art;
        async move {
            if let Some(id) = id {
                let state = h_state.read().await;
                let _ = cover_art.send(Some(state.get_cover_art_data(id).await.unwrap().webm_blob));
            } else {
                let _ = cover_art.send(None);
            }
        }
    });
    let _ = title.send(track_whole.title);
}
