use chrono::Duration;
use dioxus::{
    prelude::*,
};
use futures::StreamExt;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::HtmlAudioElement;

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
    //
    // TODO: type or runtime check this to be between 0.0 and 1.0
    SeekAbs(f32),
    // Seek relatively in the song
    SeekRel(Duration),
}

// TODO: mobile layout
#[inline_props]
#[allow(non_snake_case)]
pub fn Player<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
    // begin coroutine for actual audio player
    use_coroutine(cx, |rx| player_inner(rx));
    cx.render(rsx!{
        audio {
            id: "player-audio",
            hidden: true,
        }
        PlayerUI {}
        children
    })
}

#[inline_props]
#[allow(non_snake_case)]
fn PlayerUI(cx: Scope) -> Element {
    cx.render(rsx!{
        nav {
            class: "player-ui-wrap",
            PlayerFirstWidget {}
            PlayerControlWidget {}
            PlayerLastWidget {}
        }
    })
}

// TODO: holder of track info, and (on desktop) side menu
#[inline_props]
#[allow(non_snake_case)]
fn PlayerFirstWidget(cx: Scope) -> Element {
    cx.render(rsx!{
        div { class: "player-first-widget" }
    })
}

// TODO: holder of volume, shuffle/radio, and repeat
#[inline_props]
#[allow(non_snake_case)]
fn PlayerLastWidget(cx: Scope) -> Element {
    cx.render(rsx!{
        div { class: "player-last-widget" }
    })
}

// main music controls
#[inline_props]
#[allow(non_snake_case)]
fn PlayerControlWidget(cx: Scope) -> Element {
    let handle = use_coroutine_handle::<PlayerMsg>(cx).unwrap();
    cx.render(rsx!{
        // TODO: icons for buttons
        //
        // TODO: seeking widget
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
        }
    })
}

// TODO: impl functionality
async fn player_inner(mut rx: UnboundedReceiver<PlayerMsg>) {
    let audio_element: HtmlAudioElement =
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("player-audio")
            .unwrap()
            .dyn_into()
            .unwrap();
    while let Some(msg) = rx.next().await { }
}
