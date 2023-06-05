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

// AUTO GENERATED FILE, DO NOT EDIT. Generated by `flutter_rust_bridge`@ 1.77.0.
use crate::api::*;
use core::panic::UnwindSafe;
use flutter_rust_bridge::*;
use std::ffi::c_void;
use std::sync::Arc;

// Section: imports Section: wire functions
fn wire_new_mio_client_impl() -> support::WireSyncReturn {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap_sync(
        WrapInfo {
            debug_name: "new_mio_client",
            port: None,
            mode: FfiCallMode::Sync,
        },
        move || Ok(new_mio_client()),
    )
}

fn wire_test_set_url__method__MioClient_impl(
    port_: MessagePort,
    that: impl Wire2Api<MioClient> + UnwindSafe,
    url: impl Wire2Api<String> + UnwindSafe,
) {
    FLUTTER_RUST_BRIDGE_HANDLER.wrap(
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

// Section: wrapper structs Section: static checks Section: allocate functions
// Section: related functions Section: impl Wire2Api
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

impl Wire2Api<u8> for u8 {
    fn wire2api(self) -> u8 {
        self
    }
}

// Section: impl IntoDart
impl support::IntoDart for MioClient {
    fn into_dart(self) -> support::DartAbi {
        vec![self.0.into_dart()].into_dart()
    }
}

impl support::IntoDartExceptPrimitive for MioClient {}

// Section: executor
support::lazy_static! {
    pub static ref FLUTTER_RUST_BRIDGE_HANDLER: support:: DefaultHandler = Default:: default();
}
#[cfg(not(target_family = "wasm"))]
#[path = "bridge_generated.io.rs"]
mod io;

#[cfg(not(target_family = "wasm"))]
pub use io::*;
