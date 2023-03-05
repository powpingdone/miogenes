use crate::MioState;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use futures::StreamExt;
use log::*;
use mio_common::*;
use std::io::Read;
use tokio::fs::{
    remove_file,
    File,
    OpenOptions,
};
use tokio::io::{
    AsyncWriteExt,
    ErrorKind,
};
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new().route("/tu", put(track_upload)).route("/td", put(track_delete))
}

async fn track_upload(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(msgstructs::TrackUploadQuery { fname }): Query<msgstructs::TrackUploadQuery>,
    mut payload: BodyStream,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // TODO: store the filename for dumping purposes find a unique id for the track
    debug!("PUT /track/tu generating UUID");
    let mut uuid;
    let mut file: File;
    let mut real_fname;
    loop {
        uuid = Uuid::new_v4();
        real_fname = format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid);

        // check if file is already taken
        let check = OpenOptions::new().create_new(true).read(true).write(true).open(real_fname.clone()).await;
        match check {
            Ok(x) => {
                trace!("PUT /track/tu opened file {real_fname}");
                file = x;
                break;
            },
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("PUT /track/tu file already exists");
                    continue;
                }
                error!("PUT /track/tu failed to open file: {err}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            },
        }
    }

    // get original filename
    let orig_filename = sanitize_filename::sanitize(fname.unwrap_or_else(|| {
        trace!("PUT /track/tu generated fname with uuid");
        uuid.to_string()
    }));
    debug!("PUT /track/tu filename and uuid used: \"{orig_filename}\" -> \"{real_fname}\": {uuid}");

    // base64 decoder
    let (tx_b64, rx_b64) = std::sync::mpsc::channel();
    let (tx_byte, mut rx_byte) = tokio::sync::mpsc::unbounded_channel();
    let inner_decode = tokio::task::spawn_blocking({
        move || {
            use base64::prelude::*;
            use base64::read::DecoderReader;
            
            // used for turning a recv into a Read trait object
            struct RecvReadWrapper {
                inner: std::sync::mpsc::Receiver<u8>,
            }

            impl RecvReadWrapper {
                pub fn new(inner: std::sync::mpsc::Receiver<u8>) -> Self {
                    Self { inner }
                }
            }

            impl std::io::Read for RecvReadWrapper {
                fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                    let mut ret_len = 0;
                    for (mutbuf, byte) in buf.iter_mut().zip(self.inner.iter()) {
                        *mutbuf = byte;
                        ret_len += 1;
                    }
                    Ok(ret_len)
                }
            }

            // this task takes in the base64 encoded body and decodes it. it does so by wrapping the rx_b64
            // in a Read trait wrapper struct, and streaming the output via the Read::read function. it then
            // buffers the bytes to write out in a 1MB block and sends it off. the remainder is then sent

            // 1MB blocks
            const BUFFER_SIZE: usize = 0x10000;
            let reader = RecvReadWrapper::new(rx_b64);
            let decoder = DecoderReader::new(reader, &BASE64_URL_SAFE_NO_PAD);
            let mut buf = Vec::with_capacity(BUFFER_SIZE);
            for x in decoder.bytes() {
                buf.push(x.unwrap());
                if buf.len() >= BUFFER_SIZE {
                    tx_byte.send(buf.clone()).unwrap();
                    buf.clear();
                }
            }
            if !buf.is_empty() {
                tx_byte.send(buf).unwrap();
            }
        }
    });

    // file writer
    let inner_write = tokio::spawn(async move {
        // simple: write the vec sent over by inner_decode
        while let Some(read) = rx_byte.recv().await {
            if let Err(err) = file.write_all(&read).await {
                error!("PUT /track/tu failed to write to file: {err}");
                file.flush().await.expect("Failed to flush uploaded file: {}");
                drop(file);
                rm_file(uuid).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
        trace!("PUT /track/tu final flushing {uuid}");
        file.shutdown().await.expect("Failed to shutdown uploaded file: {}");
        Ok(())
    });

    // TODO: filesize limits
    //
    // TODO: maybe don't panic on filesystem errors(?)
    //
    // TODO: upload timeout if body stops streaming
    //
    // download the file
    while let Some(chunk) = payload.next().await {
        match chunk {
            Ok(chunk) => {
                for x in chunk {
                    if tx_b64.send(x).is_err() {
                        error!("PUT /track/tu failed to send byte during streaming chunk, something else failed...");
                        drop(tx_b64);
                        inner_decode.await.unwrap();
                        inner_write.await.unwrap()?;
                        rm_file(uuid).await;
                        return Err(StatusCode::BAD_REQUEST);
                    }
                }
            },
            // on err just delete the file
            Err(err) => {
                // delete failed upload, as well as all other uploads per this req
                error!("PUT /track/tu failure during streaming chunk: {err}");
                drop(tx_b64);
                inner_decode.await.unwrap();
                inner_write.await.unwrap()?;
                rm_file(uuid).await;
                return Err(StatusCode::BAD_REQUEST);
            },
        }
    }
    trace!("PUT /track/tu out of chunks");
    drop(tx_b64);
    trace!("PUT /track/tu closing decode");
    inner_decode.await.unwrap();
    trace!("PUT /track/tu closing write");
    inner_write.await.unwrap()?;

    // set off tasks to process files
    state.proc_tracks_tx.send((uuid, key.id, orig_filename)).unwrap();
    Ok((StatusCode::OK, Json(retstructs::UploadReturn { uuid: vec![uuid] })))
}

// rm's file when track_upload errors out
async fn rm_file(uuid: Uuid) {
    trace!("RM_FILES deleting {uuid}");
    remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid)).await.expect("unable to remove file: {}");
}

async fn track_delete(
    State(state): State<MioState>,
    Query(id): Query<msgstructs::DeleteQuery>,
    Extension(key): Extension<mio_entity::user::Model>,
) -> impl IntoResponse {
    todo!()
}
