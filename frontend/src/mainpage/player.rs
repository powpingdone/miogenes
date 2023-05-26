use crate::{
    static_assets::{
        BASE_URL,
        CLIENT,
    },
    js_sleep,
};
use chrono::Duration;
use dioxus::prelude::*;
use futures::{
    StreamExt,
    channel::mpsc,
    io::Repeat,
};
use log::*;
use mio_common::{
    msgstructs::IdInfoQuery,
    retstructs::{
        Track,
        Artist,
    },
};
use std::{
    collections::VecDeque,
    borrow::Borrow,
};
use uuid::Uuid;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    HtmlAudioElement,
    HtmlInputElement,
};
use wasm_bindgen::{
    JsCast,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShuffleType {
    NoShuffle,
    RandomNoRepeat,
    PerfectlyRandom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RepeatType {
    NoRepeat,
    RepeatQueue,
    OneTrack,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerMsg {
    // Push one track to the end of the queue
    Push(Uuid),
    // Remove one track, starting from the back of the queue.
    Rem(Option<Uuid>),
    // Play or Pause the currently playing song
    TogglePlayback,
    // Play the current song
    Play,
    // Begin force playback of "this" song. Moves queue position to that song
    ForcePlay(Uuid),
    // Pause the current song
    Pause,
    // Skip forward in the queue
    Skip,
    // Skip backwards in the queue
    SkipBack,
    // Seek absolutely to a point in the song, inclusive between 0.0 and 1.0
    SeekAbs(f64),
    // Seek relatively in the song
    SeekRel(Duration),
    // Audio element hit the end of the song
    Ended,
    // Toggle Shuffle
    Shuffle(ShuffleType),
    // Toggle repeat
    Repeat(RepeatType),
    // Volume
    Volume(f64),
}

// TODO: mobile layout
#[inline_props]
#[allow(non_snake_case)]
pub fn Player<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
    // begin coroutine for actual audio player
    let curr_playing = use_state::<Option<Uuid>>(cx, || None);
    let player_inner = use_coroutine(cx, |rx| player_inner(rx, curr_playing.to_owned()));
    cx.render(rsx!{
        audio {
            id: "player-audio",
            preload: "auto",
            hidden: true,
            onended: |evt| {
                evt.stop_propagation();
                player_inner.send(PlayerMsg::Ended);
            },
            oncanplay: |evt| {
                evt.stop_propagation();
                player_inner.send(PlayerMsg::Play);
            },
        }
        PlayerUI { current: curr_playing.to_owned() }
        children
    })
}

#[inline_props]
#[allow(non_snake_case)]
fn PlayerUI(cx: Scope, current: UseState<Option<Uuid>>) -> Element {
    cx.render(rsx!{
        nav {
            class: "player-ui-wrap",
            PlayerFirstWidget { current: current.to_owned() }
            PlayerControlWidget {}
            PlayerLastWidget {}
        }
    })
}

// TODO: holder of track info, and (on desktop) side menu
#[inline_props]
#[allow(non_snake_case)]
fn PlayerFirstWidget(cx: Scope, current: UseState<Option<Uuid>>) -> Element {
    let curr_track = use_future(cx, &current.to_owned(), |current| async move {
        if let Some(track_id) = current.get() {
            // fetch trackinfo
            //
            // TODO: error handling
            let track =
                CLIENT
                    .get(format!("{}/api/query/ti", *BASE_URL))
                    .query(&IdInfoQuery { id: *track_id })
                    .send()
                    .await
                    .unwrap()
                    .json::<Track>()
                    .await
                    .unwrap();
            let potent_coverart = track.cover_art.clone();
            let artist = track.artist.clone();
            Some((
                track,
                potent_coverart.and_then(|cover_art_id| {
                    Some(
                        format!(
                            "{}/api/query/ca?{}",
                            *BASE_URL,
                            serde_urlencoded::to_string(&IdInfoQuery { id: cover_art_id }).unwrap()
                        ),
                    )
                }),
                // blah blah async issue
                match artist {
                    Some(id) => {
                        Some(
                            CLIENT
                                .get(format!("{}/api/query/ar", *BASE_URL))
                                .query(&IdInfoQuery { id })
                                .send()
                                .await
                                .unwrap()
                                .json::<Artist>()
                                .await
                                .unwrap(),
                        )
                    },
                    None => None,
                },
            ))
        } else {
            None
        }
    });
    cx.render(rsx!{
        div {
            class: "player-first-widget",
            {
                // track info fetcher
                match curr_track.value() {
                    Some(Some((track_info, cover_art_url, artist_info))) => rsx!{
                        {
                            match cover_art_url {
                                Some(url) => rsx!{
                                    // TODO: this is temp, only for the thing to make sure it doesnt take up the whole
                                    // bar
                                    style {
                                        r#"
                                        .smol {{
                                            height: 10px;
                                            width: 10px;
                                        }}
                                        "#
                                    },
                                    img {
                                        class: "smol",
                                        src: url.as_str(),
                                    }
                                },
                                None => rsx!{
                                    // TODO: "no cover art"
                                    p {
                                        "no cover art"
                                    }
                                },
                            }
                        },
                        p {
                            format!(
                                "{} - {}",
                                track_info.title,
                                artist_info.as_ref().and_then(|artist| Some(artist.name.as_str())).unwrap_or("?")
                            )
                        }
                    },
                    Some(None) => rsx!{
                        p {
                            "Nothing is currently playing"
                        }
                    },
                    None => rsx!{
                        p {
                            "Loading..."
                        }
                    },
                }
            }
        }
    })
}

// TODO: holder of volume, shuffle/radio, and repeat
#[inline_props]
#[allow(non_snake_case)]
fn PlayerLastWidget(cx: Scope) -> Element {
    let handle = use_coroutine_handle::<PlayerMsg>(cx).unwrap();
    cx.render(rsx!{
        div { class: "player-last-widget" }
    })
}

// main music controls
#[inline_props]
#[allow(non_snake_case)]
fn PlayerControlWidget(cx: Scope) -> Element {
    let handle = use_coroutine_handle::<PlayerMsg>(cx).unwrap();

    // update the seeker
    let _seek_update = use_future(cx, (), |_| async move {
        let seeker: HtmlInputElement =
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("song-duration")
                .unwrap()
                .dyn_into()
                .unwrap();
        let audio_element: HtmlAudioElement =
            web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("player-audio")
                .unwrap()
                .dyn_into()
                .unwrap();
        loop {
            js_sleep(Duration::milliseconds(10)).await;
            seeker.set_value({
                (audio_element.current_time() / audio_element.duration()).to_string().as_ref()
            });
        }
    });
    cx.render(rsx!{
        // TODO: icons for buttons
        div {
            class: "player-control-widget",
            div {
                button {
                    onclick:| evt | {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::SkipBack)
                    },
                    "Skip Back"
                }
                button {
                    onclick:| evt | {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::SeekRel(Duration::seconds(-10)));
                    },
                    "Seek Back"
                }
                button {
                    onclick:| evt | {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::TogglePlayback);
                    },
                    "Play/Pause"
                }
                button {
                    onclick:| evt | {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::SeekRel(Duration::seconds(10)));
                    },
                    "Seek Forward"
                }
                button {
                    onclick:| evt | {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::Skip);
                    },
                    "Skip Song"
                }
            }
            div {
                input {
                    id: "song-duration",
                    r#type: "range",
                    min: "0.0",
                    max: "1.0",
                    step: "1e-7",
                    oninput: |evt| {
                        evt.stop_propagation();
                        handle.send(PlayerMsg::SeekAbs(
                            // this is dumb
                            //
                            // TODO: this also seems to crash when no song is playing. i dont know where this
                            // crashes, but it does. async rust very cool.
                            web_sys::window()
                                .unwrap()
                                .document()
                                .unwrap()
                                .get_element_by_id("song-duration")
                                .unwrap()
                                .dyn_into::<HtmlInputElement>()
                                .unwrap()
                                .value()
                                .parse()
                                .unwrap(),
                        ))
                    },
                }
            }
        }
    })
}

struct PlayerState {
    track_queue: VecDeque<Uuid>,
    track_order: VecDeque<usize>,
    curr_position: usize,
    shuffle: ShuffleType,
    repeat: RepeatType,
    audio_element: HtmlAudioElement,
}

// TODO: impl functionality
async fn player_inner(mut rx: mpsc::UnboundedReceiver<PlayerMsg>, curr_track_outer: UseState<Option<Uuid>>) {
    // setup internal state
    let mut state = PlayerState {
        track_queue: VecDeque::new(),
        track_order: VecDeque::new(),
        curr_position: 0,
        shuffle: ShuffleType::NoShuffle,
        repeat: RepeatType::NoRepeat,
        audio_element: web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("player-audio")
            .unwrap()
            .dyn_into()
            .unwrap(),
    };
    while let Some(msg) = rx.next().await {
        match msg {
            // track manipulation
            PlayerMsg::Push(id) => {
                todo!()
            },
            PlayerMsg::Rem(poss_id) => {
                if let Some(id) = poss_id {
                    todo!()
                } else {
                    todo!()
                }
            },
            PlayerMsg::ForcePlay(id) => {
                todo!()
            },
            PlayerMsg::Skip | PlayerMsg::Ended => {
                state.curr_position = state.curr_position.saturating_add(1);
                set_player(&state, &curr_track_outer);
            },
            PlayerMsg::SkipBack => {
                state.curr_position = state.curr_position.saturating_sub(1);
                set_player(&state, &curr_track_outer);
            },
            PlayerMsg::Shuffle(toggle) => {
                todo!()
            },
            PlayerMsg::Repeat(toggle) => {
                todo!()
            },
            // player manip
            PlayerMsg::TogglePlayback => {
                if state.audio_element.paused() {
                    player_play(&state.audio_element).await;
                } else {
                    // MDN seems to not say anything about pause failing, but web-sys does. sooooooo
                    // just unwrap it. possibly investigate why...
                    //
                    // maybe this is a non standard extension?
                    state.audio_element.pause().unwrap();
                }
            },
            PlayerMsg::Play => {
                player_play(&state.audio_element).await;
            },
            PlayerMsg::Pause => {
                state.audio_element.pause().unwrap();
            },
            PlayerMsg::SeekAbs(percentage) => {
                state.audio_element.set_current_time(state.audio_element.duration() * percentage.clamp(0.0, 1.0));
            },
            PlayerMsg::SeekRel(time) => {
                state.audio_element.set_current_time(state.audio_element.current_time() + time.num_seconds() as f64);
            },
            PlayerMsg::Volume(curr) => {
                state.audio_element.set_volume(curr.clamp(0.0, 1.0));
            },
        }
    }
}

// This may panic, not because of the unwrap, but because of the playback possibly
// erroring. The unwrap is used because of "weird" browsers that may not use the
// promise.
async fn player_play(audio_element: &HtmlAudioElement) {
    // the Ok returned should only be js undefined. it can be safely ignored
    if let Err(err) = JsFuture::from(audio_element.play().unwrap()).await {
        // TODO: handle errors like decoding errors
        panic!("failed to begin playing task: {err:?}");
    }
}

fn set_player( state: &PlayerState, outer_track: &UseState<Option<Uuid>>) {
    // acquire track
    let track = todo!();

    // cause side effects
    outer_track.set(track);
    match track {
        Some(track) => {
            state.audio_element.set_src(
                &format!(
                    "{}/api/track/stream?{}",
                    *BASE_URL,
                    serde_urlencoded::to_string(IdInfoQuery { id: track }).unwrap()
                ),
            );
        },
        None => {
            state.audio_element.pause().unwrap();
            state.audio_element.set_src("");
        },
    }
}
