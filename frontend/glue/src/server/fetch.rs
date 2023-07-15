use crate::{error::ErrorSplit, MioClientState};
use anyhow::anyhow;
use flutter_rust_bridge::StreamSink;
use mio_common::*;
use std::io::Read;
use uuid::Uuid;

impl MioClientState {
    // TODO: playlists
    pub fn fetch_all_albums(&self) -> Result<retstructs::Albums, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!("{}/api/load/albums", self.url)))
            .call()?
            .into_json()?)
    }

    pub fn get_track_data(&self, id: Uuid) -> Result<retstructs::Track, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_album_data(&self, id: Uuid) -> Result<retstructs::Album, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/album?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_cover_art_data(&self, id: Uuid) -> Result<retstructs::CoverArt, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/coverart?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn get_artist_data(&self, id: Uuid) -> Result<retstructs::Artist, ErrorSplit> {
        Ok(self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/query/artist?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_json()?)
    }

    pub fn stream(&self, id: Uuid, sink_back: StreamSink<Vec<u8>>) -> Result<(), ErrorSplit> {
        let mut reader = self
            .wrap_auth(self.agent.get(&format!(
                "{}/api/track?{}",
                self.url,
                serde_urlencoded::to_string(msgstructs::IdInfoQuery { id }).unwrap()
            )))
            .call()?
            .into_reader()
            .bytes();
        let mut buf = Vec::<u8>::with_capacity(16384);
        'read_loop: loop {
            // copy without reallocation
            for _ in 0..buf.capacity() {
                if let Some(byte) = reader.next() {
                    buf.push(byte?);
                } else {
                    // no more bytes
                    sink_back
                        .add(buf)
                        .then_some(())
                        .ok_or(anyhow!("sink closed before stream finished"))?;
                    sink_back.close();
                    break 'read_loop;
                }
            }

            // send back chunk
            sink_back
                .add(buf.clone())
                .then_some(())
                .ok_or(anyhow!("sink closed before stream finished"))?;
            buf.clear();
        }
        Ok(())
    }
}
