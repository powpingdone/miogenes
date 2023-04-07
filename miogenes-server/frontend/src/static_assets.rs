use std::task::Poll;
use std::pin::Pin;
use std::task::Context;
use std::future::Future;
use futures::{
    Stream,
    FutureExt,
};
use log::*;
use once_cell::sync::{
    Lazy as SyncLazy,
    OnceCell as SyncOnceCell,
};

pub static BASE_URL: SyncLazy<SyncOnceCell<String>> = SyncLazy::new(|| {
    // TODO: configure base url from server
    let cell = SyncOnceCell::new();
    cell.set({
        let url = web_sys::window().unwrap().location().origin().unwrap();
        trace!("base url is {url}");
        url
    }).unwrap();
    cell
});
type ReqFut = dyn Future<Output = Result<reqwest::Response, reqwest::Error>>;

// Struct to lazily fetch a static image.
struct StaticImg {
    url: &'static str,
    cl: Option<reqwest::Client>,
    cell: Option<&'static Vec<u8>>,
    fetch: Option<Pin<Box<ReqFut>>>,
}

impl StaticImg {
    const fn new(url: &'static str) -> Self {
        Self {
            url,
            cl: None,
            cell: None,
            fetch: None,
        }
    }
}

impl Stream for StaticImg {
    type Item = &'static Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if we have actually completed
        if self.cell.is_some() {
            return Poll::Ready(self.cell);
        }

        // init client
        if self.cl.is_none() {
            self.cl = Some({
                reqwest::Client::new()
            });
        }
        // init fetch
        if let Some(ref cl) = self.cl {
            if self.fetch.is_none() {
                self.fetch = Some(Box::pin(cl.get({
                    let mut url = BASE_URL.get().unwrap().to_owned();
                    url.push_str(&self.url);
                    url
                }).send()));
            }
        }
        if let Some(ref mut fetch) = self.fetch {
            match fetch.poll_unpin(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(ret) => todo!(),
            }
        }
        todo!()
    }
}
