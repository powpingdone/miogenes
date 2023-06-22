use super::*;
// Section: wire functions

#[no_mangle]
pub extern "C" fn wire_new_mio_client() -> support::WireSyncReturn {
    wire_new_mio_client_impl()
}

#[no_mangle]
pub extern "C" fn wire_test_set_url__method__MioClient(
    port_: i64,
    that: *mut wire_MioClient,
    url: *mut wire_uint_8_list,
) {
    wire_test_set_url__method__MioClient_impl(port_, that, url)
}

// Section: allocate functions

#[no_mangle]
pub extern "C" fn new_ArcRwLockMioClientState() -> wire_ArcRwLockMioClientState {
    wire_ArcRwLockMioClientState::new_with_null_ptr()
}

#[no_mangle]
pub extern "C" fn new_box_autoadd_mio_client_0() -> *mut wire_MioClient {
    support::new_leak_box_ptr(wire_MioClient::new_with_null_ptr())
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

// Section: impl Wire2Api

impl Wire2Api<RustOpaque<Arc<RwLock<MioClientState>>>> for wire_ArcRwLockMioClientState {
    fn wire2api(self) -> RustOpaque<Arc<RwLock<MioClientState>>> {
        unsafe { support::opaque_from_dart(self.ptr as _) }
    }
}
impl Wire2Api<String> for *mut wire_uint_8_list {
    fn wire2api(self) -> String {
        let vec: Vec<u8> = self.wire2api();
        String::from_utf8_lossy(&vec).into_owned()
    }
}
impl Wire2Api<MioClient> for *mut wire_MioClient {
    fn wire2api(self) -> MioClient {
        let wrap = unsafe { support::box_from_leak_ptr(self) };
        Wire2Api::<MioClient>::wire2api(*wrap).into()
    }
}
impl Wire2Api<MioClient> for wire_MioClient {
    fn wire2api(self) -> MioClient {
        MioClient(self.field0.wire2api())
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
pub struct wire_MioClient {
    field0: wire_ArcRwLockMioClientState,
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

// Section: sync execution mode utility

#[no_mangle]
pub extern "C" fn free_WireSyncReturn(ptr: support::WireSyncReturn) {
    unsafe {
        let _ = support::box_from_leak_ptr(ptr);
    };
}
