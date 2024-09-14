use crate::db::uuid_serialize;
use crate::db::write_transaction;
use crate::*;
use anyhow::anyhow;
use axum::http::StatusCode;
#[allow(unused)]
use log::*;
use once_cell::sync::Lazy;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use symphonia::core::audio::Channels;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tokio::io::AsyncWriteExt;
use uuid::*;

// metadata parsed from the individual file
#[derive(Default, Debug)]
struct Metadata {
    title: String,
    artist: Option<String>,
    artist_sort: Option<String>,
    album: Option<String>,
    album_sort: Option<String>,
    // TODO: support multiple images
    img: Option<(Vec<u8>, [u8; 32])>,
    disk_track: (Option<u32>, Option<u32>),
}

// TODO: upload process time limits
//
// TODO: size limits
#[tracing::instrument]
pub async fn track_upload_process(
    state: MioState,
    id: Uuid,
    path: PathBuf,
    userid: Uuid,
    orig_filename: String,
) -> Result<(), MioInnerError> {
    let mdata = tokio::task::spawn_blocking(move || get_metadata(path, orig_filename))
        .await?
        .unwrap();
    todo!()
}

#[tracing::instrument]
fn get_metadata(fname: PathBuf, orig_fname: String) -> Result<Metadata, anyhow::Error> {
    use lofty::prelude::*;
    use lofty::probe::Probe;

    // load tags
    let tags = Probe::open(fname)?.guess_file_type()?.read()?;
    let mut mdata = Metadata {
        title: orig_fname.clone(),
        ..Default::default()
    };
    // and merge
    for tag in tags.tags() {
        if orig_fname == mdata.title {
            mdata.title = tag.title().map(|x| x.into_owned()).unwrap_or(mdata.title);
        }
        mdata.artist = mdata.artist.or_else(|| tag.artist().map(Cow::into_owned));
        mdata.artist_sort = mdata.artist_sort.or_else(|| {
            tag.get(&ItemKey::TrackArtistSortOrder)
                .and_then(|x| x.value().text())
                .map(|x| x.to_owned())
        });
        mdata.album = mdata.album.or_else(|| tag.album().map(Cow::into_owned));
        mdata.album_sort = mdata.album_sort.or_else(|| {
            tag.get(&ItemKey::AlbumTitleSortOrder)
                .and_then(|x| x.value().text())
                .map(|x| x.to_owned())
        });
        mdata.disk_track.0 = mdata.disk_track.0.or_else(|| tag.disk());
        mdata.disk_track.1 = mdata.disk_track.1.or_else(|| tag.track());
        mdata.img = mdata.img.or_else(|| {
            tag.pictures()
                .get(0)
                .map(|x| (x.data().to_owned(), hash(x.data())))
        });
    }

    Ok(mdata)
}

// copy the hash of one value into a regular array since sha2 uses GenericArray's,
// this is done via just manually iterating through it
fn hash(data: &[u8]) -> [u8; 32] {
    let sha = Sha256::digest(data);
    let mut actual_hash: [u8; 32] = Default::default();
    for (hasharr, digested) in actual_hash.iter_mut().zip(sha.iter()) {
        *hasharr = *digested;
    }
    actual_hash
}

#[tracing::instrument(skip(orig))]
fn create_vec(
    orig: &[i16],
    channels: u32,
    sample_rate: u32,
    fn_dis: String,
) -> anyhow::Result<Vec<f32>> {
    use ndarray::*;

    // pad tracks shorter than 5 seconds
    let padded = {
        let full_len = sample_rate as usize * 5;
        let mut new = orig.to_vec();
        if orig.len() < full_len {
            new.extend(std::iter::repeat(0).take(full_len - orig.len()));
        }
        new
    };

    // conv to float
    let mut floated = padded
        .into_iter()
        .map(|x| x as f32 / i16::MIN as f32)
        .collect::<Vec<_>>();

    // to mono
    if channels != 1 {
        trace!("{fn_dis}: making mono");
        floated = floated
            .chunks(channels as usize)
            .map(|x| x.iter().sum::<f32>() / channels as f32)
            .collect();
    }

    // resample
    if sample_rate != 22050 {
        use rubato::Resampler;

        trace!("{fn_dis}: resampling from {sample_rate} to 22050");
        let mut new_vec = Vec::<f32>::new();
        let mut resamp =
            rubato::FftFixedIn::<f32>::new(sample_rate as usize, 22050, 4096, 2, 1).unwrap();
        let mut out_buf = vec![vec![0.0f32; resamp.output_frames_max()]; 1];
        let mut old_vec = vec![&floated[..]];
        while old_vec[0].len() >= resamp.input_frames_next() {
            let (lin, lout) = resamp
                .process_into_buffer(&old_vec, &mut out_buf, None)
                .unwrap();
            old_vec[0] = &old_vec[0][lin..];
            new_vec.extend_from_slice(&out_buf[0][..lout]);
        }
        if !old_vec[0].is_empty() {
            let (_, lout) = resamp
                .process_partial_into_buffer(Some(&old_vec), &mut out_buf, None)
                .unwrap();
            new_vec.extend_from_slice(&out_buf[0][..lout]);
        }
        floated = new_vec;
    }

    // make spectrogram
    let spec = {
        use mel_spec::prelude::*;
        use mel_spec_pipeline::*;

        debug!("{fn_dis}: making spectrogram for inference");
        let mut pipeline = Pipeline::new(PipelineConfig::new(
            // hop length is different here because, I dont know, but it makes 216 samples.
            MelConfig::new(2048, 503, 96, 22050.0),
            None,
        ));
        let handles = pipeline.start();

        // technically, it's 7.5 seconds that's the minimum, not 8, but this looks cleaner
        let range = if floated.len() > 22050 * 8 {
            floated.len() / 3..floated.len() / 3 + 22050 * 5
        } else {
            0..22050 * 5
        };
        pipeline.send_pcm(&floated[range]).unwrap();
        pipeline.close_ingress();
        let specs = pipeline.rx().into_iter().collect::<Vec<_>>();
        trace!("{fn_dis}: joining spectrogram threads");
        handles.into_iter().for_each(|x| x.join().unwrap());
        let specs = specs.into_iter().map(|x| x.1).collect::<Vec<_>>();
        let spec = ndarray::concatenate(
            ndarray::Axis(1),
            specs
                .iter()
                .map(|x| x.view())
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .unwrap();
        let shape = spec.shape().to_owned();
        trace!("{fn_dis}: shape of array is {shape:?}");
        CowArray::from(
            spec.into_shape([1, 1, shape[0], shape[1]])?
                .map(|x| *x as f32),
        )
        .into_dyn()
    };
    Ok({
        use ort::*;

        static SESSION: Lazy<InMemorySession> = Lazy::new(|| {
            trace!("creating session");

            // the majority of the binary size actually comes from this
            let model = todo!();
            let env = Environment::builder()
                .with_name("deej_ai")
                .with_log_level(LoggingLevel::Info)
                .build()
                .unwrap()
                .into_arc();
            SessionBuilder::new(&env)
                .unwrap()
                .with_parallel_execution(false)
                .unwrap()
                .with_memory_pattern(true)
                .unwrap()
                .with_model_from_memory(model)
                .unwrap()
        });
        let inp = vec![Value::from_array(SESSION.allocator(), &spec)?];
        let out = SESSION.run(inp)?;
        out.get(0)
            .ok_or_else(|| anyhow!("when picking out output, index 0 does not exist"))?
            .try_extract::<f32>()?
            .view()
            .to_slice()
            .map(|x| x.to_vec())
            .ok_or_else(|| anyhow!("arr is not contigious or in standard order"))?
    })
}

#[tracing::instrument]
async fn insert_into_db(
    db: SqlitePool,
    id: Uuid,
    userid: Uuid,
    dir: String,
    metadata: Metadata,
    orig_filename: String,
    track_vec: Vec<f32>,
) -> Result<(), MioInnerError> {
    let track_vec = track_vec
        .into_iter()
        .flat_map(|x| x.to_le_bytes())
        .collect::<Vec<_>>();
    let mut conn = db.acquire().await?;
    write_transaction(&mut conn, |txn| {
        Box::pin(async move {
            // insert cover art, check against img_hash
            let cover_art_id = {
                if let Some(cover_hash) = metadata.img.as_ref().map(|x| x.1) {
                    let q = cover_hash.as_slice();
                    match sqlx::query!(
                        "SELECT id FROM cover_art
                        WHERE img_hash = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let (img_blob, img_hash_hold) = metadata.img.unwrap();
                            let img_hash = img_hash_hold.as_slice();
                            sqlx::query!(
                                "INSERT INTO cover_art
                                (id, img_blob, img_hash)
                                VALUES (?, ?, ?);",
                                id,
                                img_blob,
                                img_hash
                            )
                            .execute(&mut *txn)
                            .await?;
                            trace!("{orig_filename}: new artist generated: {id}");
                            Some(id)
                        }
                    }
                } else {
                    None
                }
            };

            // insert artist, check on artist name
            let artist_id = {
                if let Some(q) = metadata.artist.as_ref() {
                    match sqlx::query!(
                        "SELECT id FROM artist
                        WHERE artist_name = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let artist_name = metadata.artist.unwrap();
                            sqlx::query!(
                                "INSERT INTO artist
                                (id, artist_name, sort_name)
                                VALUES (?, ?, ?);",
                                id,
                                artist_name,
                                metadata.artist_sort
                            )
                            .execute(&mut *txn)
                            .await?;
                            trace!("{orig_filename}: new artist generated: {id}");
                            Some(id)
                        }
                    }
                } else {
                    None
                }
            };

            // insert album, check on album title
            let album_id = {
                if let Some(q) = metadata.album.as_ref() {
                    match sqlx::query!(
                        "SELECT id FROM album
                        WHERE title = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let title = metadata.album.unwrap();
                            sqlx::query!(
                                "INSERT INTO album
                                (id, title, sort_title)
                                VALUES (?, ?, ?);",
                                id,
                                title,
                                metadata.album_sort
                            )
                            .execute(&mut *txn)
                            .await?;
                            trace!("{orig_filename}: new album generated: {id}");
                            Some(id)
                        }
                    }
                } else {
                    None
                }
            };

            // insert track, check on audiohash
            sqlx::query!(
                "INSERT INTO track 
                    (id,
                    title,
                    disk, 
                    track, 
                    orig_fname, 
                    album, 
                    artist, 
                    cover_art, 
                    owner,
                    path) 
                VALUES (?,?,?,?,?,?,?,?,?,?);",
                id,
                metadata.title,
                metadata.disk_track.0,
                metadata.disk_track.1,
                orig_filename,
                album_id,
                artist_id,
                cover_art_id,
                userid,
                dir
            )
            .execute(&mut *txn)
            .await?;
            trace!("{orig_filename}: new track created: {id}");
            Ok(())
        })
    })
    .await
}

// supported channel configurations
//
// TODO: why can't this be const
static POSS_CHANNELS: Lazy<[(u32, Channels); 17]> = Lazy::new(|| {
    [
        // MONO
        (1u32, Channels::FRONT_LEFT),
        // 2
        (2, Channels::FRONT_LEFT | Channels::FRONT_RIGHT),
        // 3
        (
            3,
            Channels::FRONT_LEFT | Channels::FRONT_RIGHT | Channels::FRONT_CENTRE,
        ),
        // 4
        (
            4,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::REAR_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            4,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::REAR_LEFT
                | Channels::SIDE_RIGHT,
        ),
        (
            4,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::SIDE_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            4,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::SIDE_LEFT
                | Channels::SIDE_RIGHT,
        ),
        // 5
        (
            5,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::REAR_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            5,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::REAR_LEFT
                | Channels::SIDE_RIGHT,
        ),
        (
            5,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::SIDE_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            5,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::SIDE_LEFT
                | Channels::SIDE_RIGHT,
        ),
        // 6
        (
            6,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::REAR_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            6,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::REAR_LEFT
                | Channels::SIDE_RIGHT,
        ),
        (
            6,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::SIDE_LEFT
                | Channels::REAR_RIGHT,
        ),
        (
            6,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::SIDE_LEFT
                | Channels::SIDE_RIGHT,
        ),
        // 7
        (
            7,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::REAR_CENTRE
                | Channels::SIDE_LEFT
                | Channels::SIDE_RIGHT,
        ),
        // 8
        (
            8,
            Channels::FRONT_LEFT
                | Channels::FRONT_RIGHT
                | Channels::FRONT_CENTRE
                | Channels::LFE1
                | Channels::REAR_LEFT
                | Channels::REAR_RIGHT
                | Channels::SIDE_LEFT
                | Channels::SIDE_RIGHT,
        ),
    ]
});
