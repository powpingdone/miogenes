use std::{
    collections::{
        HashSet,
        HashMap,
    },
};
use dioxus::{
    prelude::*,
};
use dioxus_router::*;
use mio_common::{
    retstructs::{
        Album,
        Track,
    },
};
use uuid::*;
use wasm_bindgen::{
    JsCast,
};
use web_sys::{
    HtmlInputElement,
};
use crate::{
    mainpage::tasks::*,
    static_assets::BASE_URL,
};
use log::*;

#[inline_props]
#[allow(non_snake_case)]
pub fn MainPage(cx: Scope, token: UseRef<Option<Uuid>>) -> Element {
    // TODO: on req fail, check if it's a failure related to auth, and if so, delete
    // the Token cookie and refresh the page
    cx.render(rsx!{
        Router {
            Route {
                to: "/",
                HomePage {}
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn HomePage(cx: Scope) -> Element {
    // hack: because we cannot pass a UseFuture to a coroutine (lifetime shenatigans
    // and whatnot), pass a tiny bit of state around to restart on change
    //
    // along with this, it's used to id' the future. why? because value() returns _a_
    // completed value, even if the future in question has been restarted
    let reset_albums = use_state::<u64>(&cx, || 0);
    let fetch_albums = use_future(&cx, reset_albums, |reset_albums| fetch_albums(reset_albums));
    let server_upload = use_coroutine(&cx, |rx| upload_to_server(rx, reset_albums.to_owned()));
    cx.render(rsx!{
        p {
            {
                match fetch_albums.value() {
                    Some(Ok((task_num, albums))) if *task_num == *reset_albums.get() => {
                        let albums = albums.iter().map(|x| rsx!{
                            Album { album_data: x }
                        });

                        rsx!{
                            albums
                        }
                    },
                    Some(Err(err)) => rsx!{
                        p {
                            "Error while getting albums: {err}"
                        }
                    },
                    _ => rsx!{
                        div {
                            hidden: true,
                            reset_albums.to_string()
                        }
                    },
                }
            }
        }
        input {
            r#type: "file",
            id: "file_upload",
            multiple: "false",
        }
        button {
            onclick: move | evt | {
                evt.stop_propagation();
                let files =
                    web_sys::window()
                        .unwrap()
                        .document()
                        .unwrap()
                        .get_element_by_id("file_upload")
                        .unwrap()
                        .dyn_ref::<HtmlInputElement>()
                        .unwrap()
                        .files()
                        .unwrap();
                server_upload.send((0 .. files.length()).map(|x| files.item(x).unwrap()).collect());
            },
            "Send over."
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn Album<'a>(cx: Scope, album_data: &'a Album) -> Element {
    // TODO: full album widget render
    let fetch = use_future(cx, *album_data, |album| {
        album_track_fetch(album)
    });
    cx.render(match fetch.value() {
        Some(Ok(tracks)) => rsx!{
            div {
                CoverArt { tracks: tracks }
                AlbumTrackList { tracks: tracks }
            }
        },
        Some(Err(err)) => rsx!{
            p {
                "Error while fetching album: {err}"
            }
        },
        None => rsx!{
            p {
                "Loading..."
            }
        },
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn AlbumTrackList<'a>(cx: Scope, tracks: &'a Vec<Track>) -> Element {
    // fetch all artist names
    //
    // TODO: Multiple albums with the same artists means that this is ideopent
    //
    // TODO: Sort by disk and track num
    let artists = use_future(cx, tracks.clone(), |tracks| async move {
        // FIXME: this seems to remove duplicately uploaded tracks
        let unique = tracks.into_iter().filter_map(|x| x.artist).collect::<HashSet<_>>();
        let ls = tokio::task::LocalSet::new();
        ls.run_until(async move {
            let client = reqwest::Client::new();
            let mut tasks = Vec::with_capacity(unique.len());
            for artist_id in unique {
                let client = client.clone();
                tasks.push(tokio::task::spawn_local(async move {
                    (
                        artist_id,
                        client
                            .get(format!("{}/api/query/ar", BASE_URL.get().unwrap()))
                            .query(&mio_common::msgstructs::IdInfoQuery { id: artist_id })
                            .send()
                            .await
                            // TODO: handle error
                            .unwrap()
                            .json::<mio_common::retstructs::Artist>()
                            .await
                            .unwrap(),
                    )
                }))
            }
            let mut ret = HashMap::with_capacity(tasks.len());
            for task in tasks {
                let task = task.await.unwrap();
                ret.insert(task.0, task.1);
            }
            ret
        }).await
    });
    cx.render(rsx!{
        div {
            for track_data in tracks.iter() {
                // TODO: make this clickable
                div {
                    p {
                        format!("{}{} - {}", {
                            match (track_data.disk, track_data.track) {
                                (_, None) => "".to_owned(),
                                (None, Some(track_num)) => format!("{track_num}. "),
                                // space is omitted here for style
                                (Some(disk), Some(track_num)) => format!("{disk}-{track_num}. "),
                            }
                        }, track_data.title, artists.value().and_then(|hmap| match track_data.artist {
                            Some(artist) => {
                                if hmap.contains_key(&artist) {
                                    Some(hmap[&artist].name.as_str())
                                } else {
                                    None
                                }
                            },
                            None => None,
                        }).unwrap_or("?"))
                    }
                }
            }
        }
    })
}

#[inline_props]
#[allow(non_snake_case)]
pub fn CoverArt<'a>(cx: Scope, tracks: &'a Vec<Track>) -> Element {
    // TODO: album with no tracks?
    //
    // TODO: albums can have multiple different cover arts
    //
    // TODO: blank image for no album art/not loaded
    let track_id = use_state(cx, || {
        tracks
            .iter()
            .find(|x| x.cover_art.is_some())
            .and_then(
                |found| Some(
                    format!(
                        "{}/api/query/ca?{}",
                        crate::BASE_URL.get().unwrap(),
                        serde_urlencoded::to_string(
                            &mio_common::msgstructs::IdInfoQuery { id: found.cover_art.unwrap() },
                        ).unwrap()
                    ),
                ),
            )
    });

    // returns link to album art
    cx.render(rsx!{
        {
            match track_id.get() {
                Some(url) => {
                    rsx!{
                        img { src: url.as_str() }
                    }
                },
                None => rsx!{
                    p {
                        "No cover art found."
                    }
                },
            }
        }
    })
}
