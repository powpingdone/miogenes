#![allow(
    non_camel_case_types,
    unused,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unit_arg,
    clippy::double_parens,
    non_snake_case,
    clippy::too_many_arguments
)]

// AUTO GENERATED FILE, DO NOT EDIT. Generated by `flutter_rust_bridge`@ 1.82.3.
use crate::api::*;
use core::panic::UnwindSafe;
use flutter_rust_bridge::rust2dart::IntoIntoDart;
use flutter_rust_bridge::*;
use std::ffi::c_void;
use std::sync::Arc;

// Section: imports
use crate::mirror::Album;
use crate::mirror::Albums;
use crate::mirror::Artist;
use crate::mirror::ClosestId;
use crate::mirror::CoverArt;
use crate::mirror::Track;
use crate::mirror::UploadReturn;
use crate::server::folder::FakeMapItem;

// Section: wire functions
fn wire_init_self_impl() -> support::WireSyncReturn {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync(
        WrapInfo {
            debug_name: "init_self",
            port: None,
            mode: FfiCallMode::Sync,
        },
        move || Result::<_, ()>::Ok(init_self()),
    )
}

fn wire_new_mio_client_impl() -> support::WireSyncReturn {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync(
        WrapInfo {
            debug_name: "new_mio_client",
            port: None,
            mode: FfiCallMode::Sync,
        },
        move || Result::<_, ()>::Ok(new_mio_client()),
    )
}

fn wire_new_player_impl(client: impl Wire2Api<MioClient> + UnwindSafe) -> support::WireSyncReturn {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync(
        WrapInfo {
            debug_name: "new_player",
            port: None,
            mode: FfiCallMode::Sync,
        },
        move || {
            let api_client = client.wire2api();
            Result::<_, ()>::Ok(new_player(api_client))
        },
    )
}

fn wire_info_stream__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "info_stream__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Stream,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| {
                Result::<_, ()>::Ok(MioPlayer::info_stream(
                    &api_that,
                    task_callback.stream_sink::<_, PStatus>(),
                ))
            }
        },
    )
}

fn wire_play__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
    id: impl Wire2Api<Option<uuid::Uuid>> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "play__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::play(&api_that, api_id))
        },
    )
}

fn wire_pause__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "pause__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::pause(&api_that))
        },
    )
}

fn wire_toggle__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "toggle__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::toggle(&api_that))
        },
    )
}

fn wire_queue__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "queue__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::queue(&api_that, api_id))
        },
    )
}

fn wire_stop__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "stop__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::stop(&api_that))
        },
    )
}

fn wire_forward__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "forward__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::forward(&api_that))
        },
    )
}

fn wire_backward__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "backward__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::backward(&api_that))
        },
    )
}

fn wire_seek__method__MioPlayer_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioPlayer> + UnwindSafe,
    ms: impl Wire2Api<u64> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "seek__method__MioPlayer",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_ms = ms.wire2api();
            move |task_callback| Result::<_, ()>::Ok(MioPlayer::seek(&api_that, api_ms))
        },
    )
}

fn wire_get_url__method__MioClient_impl(
    that: impl Wire2Api<MioClient> + UnwindSafe,
) -> support::WireSyncReturn {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync(
        WrapInfo {
            debug_name: "get_url__method__MioClient",
            port: None,
            mode: FfiCallMode::Sync,
        },
        move || {
            let api_that = that.wire2api();
            Result::<_, ()>::Ok(MioClient::get_url(&api_that))
        },
    )
}

fn wire_test_set_url__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    url: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "test_set_url__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_url = url.wire2api();
            move |task_callback| MioClient::test_set_url(&api_that, api_url)
        },
    )
}

fn wire_attempt_signup_and_login__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    username: impl Wire2Api<String> + UnwindSafe,
    password: impl Wire2Api<String> + UnwindSafe,
    password2: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "attempt_signup_and_login__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_username = username.wire2api();
            let api_password = password.wire2api();
            let api_password2 = password2.wire2api();
            move |task_callback| {
                MioClient::attempt_signup_and_login(
                    &api_that,
                    api_username,
                    api_password,
                    api_password2,
                )
            }
        },
    )
}

fn wire_attempt_login__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    username: impl Wire2Api<String> + UnwindSafe,
    password: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "attempt_login__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_username = username.wire2api();
            let api_password = password.wire2api();
            move |task_callback| MioClient::attempt_login(&api_that, api_username, api_password)
        },
    )
}

fn wire_get_albums__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_Albums, _>(
        WrapInfo {
            debug_name: "get_albums__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| MioClient::get_albums(&api_that)
        },
    )
}

fn wire_get_album__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_Album, _>(
        WrapInfo {
            debug_name: "get_album__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| MioClient::get_album(&api_that, api_id)
        },
    )
}

fn wire_get_track__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_Track, _>(
        WrapInfo {
            debug_name: "get_track__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| MioClient::get_track(&api_that, api_id)
        },
    )
}

fn wire_get_artist__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_Artist, _>(
        WrapInfo {
            debug_name: "get_artist__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| MioClient::get_artist(&api_that, api_id)
        },
    )
}

fn wire_get_cover_art__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_CoverArt, _>(
        WrapInfo {
            debug_name: "get_cover_art__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            move |task_callback| MioClient::get_cover_art(&api_that, api_id)
        },
    )
}

fn wire_get_closest_track__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    id: impl Wire2Api<uuid::Uuid> + UnwindSafe,
    ignore_tracks: impl Wire2Api<Vec<uuid::Uuid>> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_ClosestId, _>(
        WrapInfo {
            debug_name: "get_closest_track__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_id = id.wire2api();
            let api_ignore_tracks = ignore_tracks.wire2api();
            move |task_callback| MioClient::get_closest_track(&api_that, api_id, api_ignore_tracks)
        },
    )
}

fn wire_get_files_at_dir__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    path: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, Vec<String>, _>(
        WrapInfo {
            debug_name: "get_files_at_dir__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_path = path.wire2api();
            move |task_callback| MioClient::get_files_at_dir(&api_that, api_path)
        },
    )
}

fn wire_upload_file__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    fullpath: impl Wire2Api<String> + UnwindSafe,
    dir: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, mirror_UploadReturn, _>(
        WrapInfo {
            debug_name: "upload_file__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_fullpath = fullpath.wire2api();
            let api_dir = dir.wire2api();
            move |task_callback| MioClient::upload_file(&api_that, api_fullpath, api_dir)
        },
    )
}

fn wire_get_folders__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, Vec<FakeMapItem>, _>(
        WrapInfo {
            debug_name: "get_folders__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            move |task_callback| MioClient::get_folders(&api_that)
        },
    )
}

fn wire_make_dir__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    name: impl Wire2Api<String> + UnwindSafe,
    path: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap::<_, _, _, (), _>(
        WrapInfo {
            debug_name: "make_dir__method__MioClient",
            port: Some(port_),
            mode: FfiCallMode::Normal,
        },
        move || {
            let api_that = that.wire2api();
            let api_name = name.wire2api();
            let api_path = path.wire2api();
            move |task_callback| MioClient::make_dir(&api_that, api_name, api_path)
        },
    )
}

// Section: wrapper structs
#[derive(Clone)]
pub struct mirror_Album(Album);

#[derive(Clone)]
pub struct mirror_Albums(Albums);

#[derive(Clone)]
pub struct mirror_Artist(Artist);

#[derive(Clone)]
pub struct mirror_ClosestId(ClosestId);

#[derive(Clone)]
pub struct mirror_CoverArt(CoverArt);

#[derive(Clone)]
pub struct mirror_Track(Track);

#[derive(Clone)]
pub struct mirror_UploadReturn(UploadReturn);

// Section: static checks
const _: fn() = || {
    {
        let Album = None::<Album>.unwrap();
        let _: uuid::Uuid = Album.id;
        let _: String = Album.title;
        let _: Vec<uuid::Uuid> = Album.tracks;
    }
    {
        let Albums = None::<Albums>.unwrap();
        let _: Vec<uuid::Uuid> = Albums.albums;
    }
    {
        let Artist = None::<Artist>.unwrap();
        let _: uuid::Uuid = Artist.id;
        let _: String = Artist.name;
        let _: Option<String> = Artist.sort_name;
    }
    {
        let ClosestId = None::<ClosestId>.unwrap();
        let _: uuid::Uuid = ClosestId.id;
        let _: f32 = ClosestId.similarity;
    }
    {
        let CoverArt = None::<CoverArt>.unwrap();
        let _: uuid::Uuid = CoverArt.id;
        let _: Vec<u8> = CoverArt.webm_blob;
    }
    {
        let Track = None::<Track>.unwrap();
        let _: uuid::Uuid = Track.id;
        let _: Option<uuid::Uuid> = Track.album;
        let _: Option<uuid::Uuid> = Track.cover_art;
        let _: Option<uuid::Uuid> = Track.artist;
        let _: String = Track.title;
        let _: Option<i64> = Track.disk;
        let _: Option<i64> = Track.track;
    }
    {
        let UploadReturn = None::<UploadReturn>.unwrap();
        let _: uuid::Uuid = UploadReturn.uuid;
    }
};

// Section: allocate functions Section: related functions Section: impl Wire2Api
pub trait Wire2Api<T> {
    fn wire2api(self) -> T;
}

impl<T, S> Wire2Api<Option<T>> for *mut S
where
    *mut S: Wire2Api<T>,
{
    fn wire2api(self) -> Option<T> {
        (!self.is_null()).then(|| self.wire2api())
    }
}

impl Wire2Api<u64> for u64 {
    fn wire2api(self) -> u64 {
        self
    }
}

impl Wire2Api<u8> for u8 {
    fn wire2api(self) -> u8 {
        self
    }
}

// Section: impl IntoDart
impl support::IntoDart for mirror_Album {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.0.id.into_into_dart().into_dart(),
            self.0.title.into_into_dart().into_dart(),
            self.0.tracks.into_into_dart().into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_Album {}

impl rust2dart::IntoIntoDart<mirror_Album> for Album {
    fn into_into_dart(self) -> mirror_Album {
        mirror_Album(self)
    }
}

impl support::IntoDart for mirror_Albums {
    fn into_dart(self) -> support::DartAbi {
        vec![self.0.albums.into_into_dart().into_dart()].into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_Albums {}

impl rust2dart::IntoIntoDart<mirror_Albums> for Albums {
    fn into_into_dart(self) -> mirror_Albums {
        mirror_Albums(self)
    }
}

impl support::IntoDart for mirror_Artist {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.0.id.into_into_dart().into_dart(),
            self.0.name.into_into_dart().into_dart(),
            self.0.sort_name.into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_Artist {}

impl rust2dart::IntoIntoDart<mirror_Artist> for Artist {
    fn into_into_dart(self) -> mirror_Artist {
        mirror_Artist(self)
    }
}

impl support::IntoDart for mirror_ClosestId {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.0.id.into_into_dart().into_dart(),
            self.0.similarity.into_into_dart().into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_ClosestId {}

impl rust2dart::IntoIntoDart<mirror_ClosestId> for ClosestId {
    fn into_into_dart(self) -> mirror_ClosestId {
        mirror_ClosestId(self)
    }
}

impl support::IntoDart for mirror_CoverArt {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.0.id.into_into_dart().into_dart(),
            self.0.webm_blob.into_into_dart().into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_CoverArt {}

impl rust2dart::IntoIntoDart<mirror_CoverArt> for CoverArt {
    fn into_into_dart(self) -> mirror_CoverArt {
        mirror_CoverArt(self)
    }
}

impl support::IntoDart for DecoderStatus {
    fn into_dart(self) -> support::DartAbi {
        match self {
            Self::Playing => 0,
            Self::Paused => 1,
            Self::Buffering => 2,
            Self::Loading => 3,
            Self::Dead => 4,
        }
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for DecoderStatus {}

impl rust2dart::IntoIntoDart<DecoderStatus> for DecoderStatus {
    fn into_into_dart(self) -> Self {
        self
    }
}

impl support::IntoDart for FakeMapItem {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.key.into_into_dart().into_dart(),
            self.value.into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for FakeMapItem {}

impl rust2dart::IntoIntoDart<FakeMapItem> for FakeMapItem {
    fn into_into_dart(self) -> Self {
        self
    }
}

impl support::IntoDart for MioClient {
    fn into_dart(self) -> support::DartAbi {
        vec![self.0.into_dart()].into_dart()
    }
}

impl support::IntoDartExceptPrimitive for MioClient {}

impl rust2dart::IntoIntoDart<MioClient> for MioClient {
    fn into_into_dart(self) -> Self {
        self
    }
}

impl support::IntoDart for MioPlayer {
    fn into_dart(self) -> support::DartAbi {
        vec![self.0.into_dart()].into_dart()
    }
}

impl support::IntoDartExceptPrimitive for MioPlayer {}

impl rust2dart::IntoIntoDart<MioPlayer> for MioPlayer {
    fn into_into_dart(self) -> Self {
        self
    }
}

impl support::IntoDart for PStatus {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.err_msg.into_dart(),
            self.queue.into_into_dart().into_dart(),
            self.status.into_dart(),
            self.curr_playing.into_dart(),
            self.playback_pos_s.into_into_dart().into_dart(),
            self.playback_pos_ms.into_into_dart().into_dart(),
            self.playback_len_s.into_into_dart().into_dart(),
            self.playback_len_ms.into_into_dart().into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for PStatus {}

impl rust2dart::IntoIntoDart<PStatus> for PStatus {
    fn into_into_dart(self) -> Self {
        self
    }
}

impl support::IntoDart for mirror_Track {
    fn into_dart(self) -> support::DartAbi {
        vec![
            self.0.id.into_into_dart().into_dart(),
            self.0.album.into_dart(),
            self.0.cover_art.into_dart(),
            self.0.artist.into_dart(),
            self.0.title.into_into_dart().into_dart(),
            self.0.disk.into_dart(),
            self.0.track.into_dart(),
        ]
        .into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_Track {}

impl rust2dart::IntoIntoDart<mirror_Track> for Track {
    fn into_into_dart(self) -> mirror_Track {
        mirror_Track(self)
    }
}

impl support::IntoDart for mirror_UploadReturn {
    fn into_dart(self) -> support::DartAbi {
        vec![self.0.uuid.into_into_dart().into_dart()].into_dart()
    }
}

impl support::IntoDartExceptPrimitive for mirror_UploadReturn {}

impl rust2dart::IntoIntoDart<mirror_UploadReturn> for UploadReturn {
    fn into_into_dart(self) -> mirror_UploadReturn {
        mirror_UploadReturn(self)
    }
}

// Section: executor
support::lazy_static! {
    pub static ref FLUTTER_RUST_BRIDGE_HANDLER: support:: DefaultHandler = Default:: default();
}
#[cfg(not(target_family = "wasm"))]
#[path = "bridge_generated.io.rs"]
mod io;

#[cfg(not(target_family = "wasm"))]
pub use io::*;
