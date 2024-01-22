use core::slice::SlicePattern;
use std::{fmt::write, io::Cursor, pin::Pin};

use image::{GenericImageView, RgbaImage};
use mio_glue::player::{CurrentlyDecoding, Player};
use slint::{Rgba8Pixel, SharedPixelBuffer};
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
            let new_state = rx.borrow_and_update();
            let (is_new_state, is_track_diff) = state.set_new_decoding(&*new_state);
            if is_new_state {
                if let Ok(app) = self.w_app() {
                    // main setup loop
                    let cb = app.global::<crate::PlayerCB>();
                    if is_track_diff {
                        // invalidate UI metadata
                        cb.set_loaded(false);
                        // spawn task to fetch new metadata
                        let rt = self.w_rt().unwrap();
                        rt.spawn(state.new_fetch_task(new_state.curr));
                    }

                    // set fetched mdata
                    if state.is_all_mdata_ready() && !cb.get_loaded() {
                        cb.set_loaded(true);
                        cb.set_title(state.title.as_ref().unwrap().into());
                        cb.set_album(
                            state
                                .album
                                .as_ref()
                                .map(|x| x.as_str())
                                .unwrap_or("?")
                                .into(),
                        );
                        cb.set_artist(
                            state
                                .artist
                                .as_ref()
                                .map(|x| x.as_str())
                                .unwrap_or("?")
                                .into(),
                        );
                        cb.set_cover_art(
                            image_get(state.cover_art.as_ref().map(|x| x.as_slice()))
                                .map(|img| {
                                    slint::Image::from_rgba8(
                                        slint::SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                                            img.as_raw(),
                                            img.width(),
                                            img.height(),
                                        ),
                                    )
                                })
                                .unwrap_or_else(|_| {
                                    slint::Image::from_rgba8(slint::SharedPixelBuffer::new(0, 0))
                                }),
                        );
                    }
                    // set currently decoding metadata
                    todo!()
                } else {
                    return;
                }
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

    pub fn new_fetch_task(&mut self, id: Uuid) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        // future for recv mdata
        let (title_tx, title_rx) = oneshot::channel();
        let (album_tx, album_rx) = oneshot::channel();
        let (artist_tx, artist_rx) = oneshot::channel();
        let (cover_art_tx, cover_art_rx) = oneshot::channel();
        let fut = Box::pin(fetch_track_mdata(
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

async fn fetch_track_mdata(
    title: oneshot::Sender<String>,
    album: oneshot::Sender<Option<String>>,
    artist: oneshot::Sender<Option<String>>,
    cover_art: oneshot::Sender<Option<Vec<u8>>>,
) {
    todo!()
}
