use crate::*;
use slint::{ComponentHandle, ModelExt, ModelRc, Rgba8Pixel, SharedPixelBuffer, VecModel};
use std::{collections::HashMap, time::Duration};

impl MioFrontendWeak {
    pub fn start_album_poll_task(&self) -> tokio::task::JoinHandle<()> {
        self.w_rt().unwrap().spawn(self.clone().album_poll_task())
    }

    async fn album_poll_task(self) {
        loop {
            // go to sleep, or wait for a wake up from the uploader
            //
            // TODO: do uploader wake
            match tokio::time::timeout(Duration::from_millis(500), futures::future::pending::<()>())
                .await
            {
                Ok(()) => {
                    drop(self.app.upgrade_in_event_loop(|app| {
                        app.global::<AlbumsCB>().set_albums_setup(false)
                    }))
                }
                Err(_) => {
                    if let Ok(app) = self.w_app() {
                        let cb = app.global::<AlbumsCB>();
                        if cb.get_albums_setup() {
                            continue;
                        }
                    }
                }
            }

            // alright, we got a real request, fill the album
            self.app
                .upgrade_in_event_loop(|app| {
                    // clear the thing
                    app.global::<AlbumsCB>()
                        .set_all_albums(ModelRc::new(VecModel::default()));
                })
                .unwrap();
            let h_state = self.w_state().unwrap();
            let state = h_state.read().await;

            // check if logged in
            if state.key.get().is_none() {
                continue;
            }

            // spawn tasks
            let all_albums = match state.fetch_all_albums().await.map(|x| x.albums) {
                Ok(x) => x,
                // TODO: log errors
                Err(_) => continue,
            };
            let mut tasks = Vec::with_capacity(all_albums.len());
            for id in all_albums {
                tasks.push(tokio::spawn({
                    let h_state = h_state.clone();
                    async move {
                        let state = h_state.read().await;
                        state.get_album_data(id.to_owned()).await
                    }
                }));
            }

            // collect tasks
            let mut ret = Vec::with_capacity(tasks.capacity());
            for task in tasks.into_iter() {
                // TODO: errors
                ret.push(task.await.unwrap().unwrap());
            }
            ret.sort_unstable_by(|a, b| a.title.cmp(&b.title));

            // mapping
            let uuid_pos = Arc::new(
                ret.iter()
                    .map(|x| x.id.to_owned())
                    .enumerate()
                    .map(|(e, x)| (x, e))
                    .collect::<HashMap<_, _>>(),
            );

            // update ui
            {
                let new_album_list = VecModel::default();
                new_album_list.set_vec(Vec::with_capacity(ret.capacity()));
                for album in ret.iter() {
                    new_album_list.push(AlbumMetadata {
                        id: album.id.into(),
                        img_loaded: false,
                        name: album.title.to_owned().into(),
                        img: slint::Image::default(),
                    });
                }
                let new = ModelRc::new(new_album_list);
                self.w_app()
                    .unwrap()
                    .global::<AlbumsCB>()
                    .set_all_albums(new);
                drop(
                    self.app.upgrade_in_event_loop(|app| {
                        app.global::<AlbumsCB>().set_albums_setup(true)
                    }),
                );
            }

            // setup the image tasks
            let mut cover_art = Vec::with_capacity(ret.capacity());
            for album in ret.into_iter() {
                if let Some(tr_id) = album.tracks.get(0) {
                    let album_id = album.id;
                    let tr_id = tr_id.clone();
                    cover_art.push(tokio::spawn({
                        let w_state = self.clone();
                        let uuid_pos = uuid_pos.clone();
                        async move {
                            let h_state = w_state.w_state().unwrap();
                            let state = h_state.read().await;
                            let x = async {
                                state
                                    .get_cover_art_data(
                                        // TODO: ERRORS EEE
                                        state.get_track_data(tr_id.clone()).await.ok()?.cover_art?,
                                    )
                                    .await
                                    .ok()
                                    .map(|x| (album_id, x))
                            }
                            .await;
                            if let Some((album_id, x)) = x {
                                let Ok(img) = image::load_from_memory(&x.webm_blob) else {
                                    return;
                                };
                                let img = img.thumbnail_exact(256, 256).into_rgba8();
                                drop(w_state.app.upgrade_in_event_loop(move |app| {
                                    let img = slint::Image::from_rgba8(SharedPixelBuffer::<
                                        Rgba8Pixel,
                                    >::clone_from_slice(
                                        img.as_raw(),
                                        img.width(),
                                        img.height(),
                                    ));
                                    let albums = app.global::<AlbumsCB>().get_all_albums();
                                    albums.row_data_tracked(uuid_pos[&album_id]).unwrap().img = img;
                                }));
                            }
                        }
                    }));
                }
            }

            // and wait for all image tasks to finish
            for task in cover_art.into_iter() {
                drop(task.await);
            }
        }
    }
}
