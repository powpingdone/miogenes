use super::*;
// Section: wire functions

#[no_mangle]
pub extern "C" fn wire_init_self() -> support::WireSyncReturn {
    wire_init_self_impl()
}

#[no_mangle]
pub extern "C" fn wire_new_mio_client() -> support::WireSyncReturn {
    wire_new_mio_client_impl()
}

#[no_mangle]
pub extern "C" fn wire_new_player(client: *mut wire_MioClient) -> support::WireSyncReturn {
    wire_new_player_impl(client)
}

#[no_mangle]
pub extern "C" fn wire_info_stream__method__MioPlayer(port_: i64, that: *mut wire_MioPlayer) {
    wire_info_stream__method__MioPlayer_impl(port_, that)
}

#[no_mangle]
pub extern "C" fn wire_play__method__MioPlayer(
    that: *mut wire_MioPlayer,
    id: *mut wire_uint_8_list,
) -> support::WireSyncReturn {
    wire_play__method__MioPlayer_impl(that, id)
}

#[no_mangle]
pub extern "C" fn wire_pause__method__MioPlayer(
    that: *mut wire_MioPlayer,
) -> support::WireSyncReturn {
    wire_pause__method__MioPlayer_impl(that)
}

#[no_mangle]
pub extern "C" fn wire_toggle__method__MioPlayer(
    that: *mut wire_MioPlayer,
) -> support::WireSyncReturn {
    wire_toggle__method__MioPlayer_impl(that)
}

#[no_mangle]
pub extern "C" fn wire_queue__method__MioPlayer(
    that: *mut wire_MioPlayer,
    id: *mut wire_uint_8_list,
) -> support::WireSyncReturn {
    wire_queue__method__MioPlayer_impl(that, id)
}

#[no_mangle]
pub extern "C" fn wire_unqueue__method__MioPlayer(
    that: *mut wire_MioPlayer,
    id: *mut wire_uint_8_list,
) -> support::WireSyncReturn {
    wire_unqueue__method__MioPlayer_impl(that, id)
}

#[no_mangle]
pub extern "C" fn wire_stop__method__MioPlayer(
    that: *mut wire_MioPlayer,
) -> support::WireSyncReturn {
    wire_stop__method__MioPlayer_impl(that)
}

#[no_mangle]
pub extern "C" fn wire_forward__method__MioPlayer(
    that: *mut wire_MioPlayer,
) -> support::WireSyncReturn {
    wire_forward__method__MioPlayer_impl(that)
}

#[no_mangle]
pub extern "C" fn wire_volume__method__MioPlayer(
    that: *mut wire_MioPlayer,
    volume: f32,
) -> support::WireSyncReturn {
    wire_volume__method__MioPlayer_impl(that, volume)
}

#[no_mangle]
pub extern "C" fn wire_get_url__method__MioClient(
    that: *mut wire_MioClient,
) -> support::WireSyncReturn {
    wire_get_url__method__MioClient_impl(that)
}

#[no_mangle]
pub extern "C" fn wire_test_set_url__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    url: *mut wire_uint_8_list,
) {
    wire_test_set_url__method__MioClient_impl(port_, that, url)
}

#[no_mangle]
pub extern "C" fn wire_attempt_signup_and_login__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    username: *mut wire_uint_8_list,
    password: *mut wire_uint_8_list,
    password2: *mut wire_uint_8_list,
) {
    wire_attempt_signup_and_login__method__MioClient_impl(
        port_, that, username, password, password2,
    )
}

#[no_mangle]
pub extern "C" fn wire_attempt_login__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    username: *mut wire_uint_8_list,
    password: *mut wire_uint_8_list,
) {
    wire_attempt_login__method__MioClient_impl(port_, that, username, password)
}

#[no_mangle]
pub extern "C" fn wire_get_albums__method__MioClient(port_: i64, that: *mut wire_MioClient) {
    wire_get_albums__method__MioClient_impl(port_, that)
}

#[no_mangle]
pub extern "C" fn wire_get_album__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    id: *mut wire_uint_8_list,
) {
    wire_get_album__method__MioClient_impl(port_, that, id)
}

#[no_mangle]
pub extern "C" fn wire_get_track__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    id: *mut wire_uint_8_list,
) {
    wire_get_track__method__MioClient_impl(port_, that, id)
}

#[no_mangle]
pub extern "C" fn wire_get_artist__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    id: *mut wire_uint_8_list,
) {
    wire_get_artist__method__MioClient_impl(port_, that, id)
}

#[no_mangle]
pub extern "C" fn wire_get_cover_art__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    id: *mut wire_uint_8_list,
) {
    wire_get_cover_art__method__MioClient_impl(port_, that, id)
}

#[no_mangle]
pub extern "C" fn wire_get_closest_track__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    id: *mut wire_uint_8_list,
    ignore_tracks: *mut wire_uint_8_list,
) {
    wire_get_closest_track__method__MioClient_impl(port_, that, id, ignore_tracks)
}

#[no_mangle]
pub extern "C" fn wire_get_files_at_dir__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    path: *mut wire_uint_8_list,
) {
    wire_get_files_at_dir__method__MioClient_impl(port_, that, path)
}

#[no_mangle]
pub extern "C" fn wire_upload_file__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    fullpath: *mut wire_uint_8_list,
    dir: *mut wire_uint_8_list,
) {
    wire_upload_file__method__MioClient_impl(port_, that, fullpath, dir)
}

#[no_mangle]
pub extern "C" fn wire_get_folders__method__MioClient(port_: i64, that: *mut wire_MioClient) {
    wire_get_folders__method__MioClient_impl(port_, that)
}

#[no_mangle]
pub extern "C" fn wire_make_dir__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    name: *mut wire_uint_8_list,
    path: *mut wire_uint_8_list,
) {
    wire_make_dir__method__MioClient_impl(port_, that, name, path)
}

// Section: allocate functions

#[no_mangle]
pub extern "C" fn new_ArcRwLockMioClientState() -> wire_ArcRwLockMioClientState {
    wire_ArcRwLockMioClientState::new_with_null_ptr()
}

#[no_mangle]
pub extern "C" fn new_Player() -> wire_Player {
    wire_Player::new_with_null_ptr()
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_mio_client_0() -> *mut wire_MioClient {
    support::new_leak_box_ptr(wire_MioClient::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_mio_player_0() -> *mut wire_MioPlayer {
    support::new_leak_box_ptr(wire_MioPlayer::new_with_null_ptr())
}

#[no_mangle]
pub extern "C" fn new_uint_8_list_0(len: i32) -> *mut wire_uint_8_list {
    let ans = wire_uint_8_list {
        ptr: support::new_leak_vec_ptr(Default::default(), len),
        len,
    };
    support::new_leak_box_ptr(ans)
}

// Section: related functions

#[no_mangle]
pub extern "C" fn drop_opaque_ArcRwLockMioClientState(ptr: *const c_void) {
    unsafe {
        Arc::<Arc<RwLock<MioClientState>>>::decrement_strong_count(ptr as _);
    }
}

#[no_mangle]
pub extern "C" fn share_opaque_ArcRwLockMioClientState(ptr: *const c_void) -> *const c_void {
    unsafe {
        Arc::<Arc<RwLock<MioClientState>>>::increment_strong_count(ptr as _);
        ptr
    }
}

#[no_mangle]
pub extern "C" fn drop_opaque_Player(ptr: *const c_void) {
    unsafe {
        Arc::<Player>::decrement_strong_count(ptr as _);
    }
}

#[no_mangle]
pub extern "C" fn share_opaque_Player(ptr: *const c_void) -> *const c_void {
    unsafe {
        Arc::<Player>::increment_strong_count(ptr as _);
        ptr
    }
}

// Section: impl Wire2Api

impl Wire2Api<RustOpaque<Arc<RwLock<MioClientState>>>> for wire_ArcRwLockMioClientState {
    fn wire2api(self) -> RustOpaque<Arc<RwLock<MioClientState>>> {
        unsafe { support::opaque_from_dart(self.ptr as _) }
    }
}
impl Wire2Api<RustOpaque<Player>> for wire_Player {
    fn wire2api(self) -> RustOpaque<Player> {
        unsafe { support::opaque_from_dart(self.ptr as _) }
    }
}
impl Wire2Api<String> for *mut wire_uint_8_list {
    fn wire2api(self) -> String {
        let vec: Vec<u8> = self.wire2api();
        String::from_utf8_lossy(&vec).into_owned()
    }
}
impl Wire2Api<uuid::Uuid> for *mut wire_uint_8_list {
    fn wire2api(self) -> uuid::Uuid {
        let single: Vec<u8> = self.wire2api();
        wire2api_uuid_ref(single.as_slice())
    }
}
impl Wire2Api<Vec<uuid::Uuid>> for *mut wire_uint_8_list {
    fn wire2api(self) -> Vec<uuid::Uuid> {
        let multiple: Vec<u8> = self.wire2api();
        wire2api_uuids(multiple)
    }
}
impl Wire2Api<MioClient> for *mut wire_MioClient {
    fn wire2api(self) -> MioClient {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<MioClient>::wire2api(*wrap).into()
    }
}
impl Wire2Api<MioPlayer> for *mut wire_MioPlayer {
    fn wire2api(self) -> MioPlayer {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<MioPlayer>::wire2api(*wrap).into()
    }
}

impl Wire2Api<MioClient> for wire_MioClient {
    fn wire2api(self) -> MioClient {
        MioClient(self.field0.wire2api())
    }
}
impl Wire2Api<MioPlayer> for wire_MioPlayer {
    fn wire2api(self) -> MioPlayer {
        MioPlayer(self.field0.wire2api())
    }
}

impl Wire2Api<Vec<u8>> for *mut wire_uint_8_list {
    fn wire2api(self) -> Vec<u8> {
        unsafe {
            let wrap = support::box_from_leak_ptr(self);
            support::vec_from_leak_ptr(wrap.ptr, wrap.len)
        }
    }
}
// Section: wire structs

#[repr(C)]
#[derive(Clone)]
pub struct wire_ArcRwLockMioClientState {
    ptr: *const core::ffi::c_void,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_Player {
    ptr: *const core::ffi::c_void,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_MioClient {
    field0: wire_ArcRwLockMioClientState,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_MioPlayer {
    field0: wire_Player,
}

#[repr(C)]
#[derive(Clone)]
pub struct wire_uint_8_list {
    ptr: *mut u8,
    len: i32,
}

// Section: impl NewWithNullPtr

pub trait NewWithNullPtr {
    fn new_with_null_ptr() -> Self;
}

impl<T> NewWithNullPtr for *mut T {
    fn new_with_null_ptr() -> Self {
        std::ptr::null_mut()
    }
}

impl NewWithNullPtr for wire_ArcRwLockMioClientState {
    fn new_with_null_ptr() -> Self {
        Self {
            ptr: core::ptr::null(),
        }
    }
}
impl NewWithNullPtr for wire_Player {
    fn new_with_null_ptr() -> Self {
        Self {
            ptr: core::ptr::null(),
        }
    }
}

impl NewWithNullPtr for wire_MioClient {
    fn new_with_null_ptr() -> Self {
        Self {
            field0: wire_ArcRwLockMioClientState::new_with_null_ptr(),
        }
    }
}

impl Default for wire_MioClient {
    fn default() -> Self {
        Self::new_with_null_ptr()
    }
}

impl NewWithNullPtr for wire_MioPlayer {
    fn new_with_null_ptr() -> Self {
        Self {
            field0: wire_Player::new_with_null_ptr(),
        }
    }
}

impl Default for wire_MioPlayer {
    fn default() -> Self {
        Self::new_with_null_ptr()
    }
}

// Section: sync execution mode utility

#[no_mangle]
pub extern "C" fn free_WireSyncReturn(ptr: support::WireSyncReturn) {
    unsafe {
        let _ = support::box_from_leak_ptr(ptr);
    };
}
