use std::sync::OnceLock;
use reqwest::Client;

pub mod error;
mod player;
mod server;

// https://github.com/RustAudio/cpal/issues/720#issuecomment-1311813294
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn JNI_OnLoad(vm: jni::JavaVM, res: *mut std::os::raw::c_void) -> jni::sys::jint {
    use std::ffi::c_void;

    let vm = vm.get_java_vm_pointer() as *mut c_void;
    unsafe {
        ndk_context::initialize_android_context(vm, res);
    }
    jni::JNIVersion::V6.into()
}

// The second half of the connections. This actually sends out the raw connections
// to the server and also handles the state for connecting to it.
#[derive(Debug)]
pub struct MioClientState {
    url: String,
    agent: Client,
    pub key: OnceLock<mio_common::auth::JWT>,
}

// TODO: make client making connections redundant. Ie: any connection that fails
// for a reason like "no connection to host" should retry after some metrics.
impl MioClientState {
    pub fn new() -> Self {
        Self {
            url: "".to_owned(),
            key: OnceLock::new(),
            agent: Client::new()
        }
    }

    // wrapper function. adds the auth header to the current request
    fn wrap_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(self.key.get().unwrap().to_string())
    }
}

impl Default for MioClientState {
    fn default() -> Self {
        Self::new()
    }
}
