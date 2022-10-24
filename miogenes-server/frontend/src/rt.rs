use js_sys::Promise;
use log::*;
use oneshot as osh;
use osh::TryRecvError::*;
use wasm_bindgen::JsValue;

use std::{future::IntoFuture, sync::Arc};

pub struct RunTime {
    futures: Vec<(Arc<osh::Receiver<()>>, Promise)>,
}

impl RunTime {
    pub fn new() -> Self {
        RunTime { futures: vec![] }
    }

    pub fn push_future<F, E>(&mut self, f: F)
    where
        F: IntoFuture + 'static,
        F::Output: Into<Result<(), E>>,
        E: ToString,
    {
        let (tx, rx) = osh::channel();
        self.futures.push((
            Arc::new(rx),
            wasm_bindgen_futures::future_to_promise(async move {
                let x = f.await.into();
                tx.send(()).unwrap();
                match x {
                    Ok(_) => Ok(JsValue::NULL),
                    Err(err) => {
                        error!("{}", err.to_string());
                        Ok(JsValue::NULL)
                    }
                }
            }),
        ));
        self.cleaner();
    }

    fn cleaner(&mut self) {
        self.futures = self
            .futures
            .iter()
            .cloned()
            .filter(|(recv, _)| match recv.try_recv() {
                Ok(_) => false,
                Err(Empty) => true,
                Err(Disconnected) => panic!("broken oneshot channel detected in runtime"),
            })
            .collect::<Vec<_>>();
    }
}
