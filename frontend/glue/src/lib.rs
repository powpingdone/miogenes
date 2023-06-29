mod bridge_generated;

use std::sync::OnceLock;
use ureq::Agent;

mod api;
mod error;
mod mirror;
mod server;

// The second half of the connections. This actually sends out the raw connections
// to the server and also handles the state for connecting to it.
#[derive(Debug)]
pub struct MioClientState {
    url: String,
    agent: Agent,
    pub key: OnceLock<mio_common::auth::JWT>,
}

impl Default for MioClientState {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: make client making connections redundant. Ie: any connection that fails
// for a reason like "no connection to host" should retry after some metrics.
impl MioClientState {
    pub fn new() -> Self {
        Self {
            url: "".to_owned(),
            agent: ureq::agent(),
            key: OnceLock::new(),
        }
    }

    // wrapper function. adds the auth header to the current request
    fn wrap_auth(&self, req: ureq::Request) -> ureq::Request {
        req.set(
            "Authorization",
            &format!("Bearer {}", self.key.get().unwrap().to_string()),
        )
    }
}
