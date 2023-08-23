use crate::db::uuid_serialize;
use crate::db::write_transaction;
use crate::*;
use anyhow::anyhow;
use axum::http::StatusCode;
use glib::SendValue;
use gstreamer::glib;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
#[allow(unused)]
use log::*;
use once_cell::sync::Lazy;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
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
#[derive(Default)]
struct Metadata {
    title: String,
    other_tags: String,
    artist: Option<String>,
    artist_sort: Option<String>,
    album: Option<String>,
    album_sort: Option<String>,
    img: Option<(Vec<u8>, [u8; 32])>,
    disk_track: (Option<i32>, Option<i32>),
}

// TODO: upload process time limits
//
// TODO: size limits
pub async fn track_upload_process(
    state: MioState,
    id: Uuid,
    path: PathBuf,
    dir: String,
    userid: Uuid,
    orig_filename: String,
) -> Result<(), MioInnerError> {
    // process metadata
    let (mdata, track_vec, encoded) = tokio::task::spawn_blocking({
        let orig_filename = orig_filename.clone();
        let path = path.clone();
        move || {
            let mdata = get_metadata(path.clone(), orig_filename.clone())?;

            // get waveform & desc
            let (desc, waveform) = create_qoa(path, orig_filename)
                .map_err(|err| MioInnerError::TrackProcessingError(err, StatusCode::BAD_REQUEST))?;

            // generate vec
            let track_vec =
                create_vec(&waveform, desc.channels, desc.sample_rate).map_err(|err| {
                    MioInnerError::TrackProcessingError(err, StatusCode::INTERNAL_SERVER_ERROR)
                })?;

            // conv into Quite Ok Audio
            let encoded = mio_qoa_impl::encode(waveform, &desc).map_err(|err| {
                MioInnerError::TrackProcessingError(err.into(), StatusCode::BAD_REQUEST)
            })?;
            Ok((mdata, track_vec, encoded))
        }
    })
    .await
    .map_err(|err| {
        MioInnerError::TrackProcessingError(
            anyhow!("failed to run task: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?
    .map_err(|err| MioInnerError::TrackProcessingError(err, StatusCode::BAD_REQUEST))?;

    // write out new encoded file
    trace!("{orig_filename}: writing out encoding");
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .await?;
    file.write_all(&encoded.raw_file).await?;
    file.sync_all().await?;
    drop(file);

    // insert into the database
    insert_into_db(state.db, id, userid, dir, mdata, orig_filename, track_vec).await
}

fn get_metadata(fname: PathBuf, orig_path: String) -> Result<Metadata, anyhow::Error> {
    // TODO: make this timeout configurable
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(10))?;
    trace!("{orig_path}: creating discoverer");
    let fname = glib::filename_to_uri(Path::new(&fname).absolutize()?, None)?;
    trace!("{orig_path}: new uri created: '{fname}'");
    debug!("{orig_path}: begin discovery");
    let data = discover.discover_uri(&fname)?;

    // TODO: implement errors for the rest (possibly, may not be needed)
    trace!("{orig_path}: result: {:?}", data.result());
    match data.result() {
        DiscovererResult::Ok => (),
        DiscovererResult::MissingPlugins => {
            anyhow::bail!("Missing plugin needed for file {orig_path}");
        }
        DiscovererResult::Timeout => {
            anyhow::bail!("Timeout reached for processing tags");
        }
        // these branches _shouldn't_ fail.
        //
        // Busy -> each discoverer is in it's own thread, where it only reads one file
        DiscovererResult::Busy => unimplemented!(),
        // UriInvalid -> fname is produced via glib::filename_to_uri
        DiscovererResult::UriInvalid => unimplemented!(),
        // Error -> discover_uri can return an error, so this shouldn't happen here
        DiscovererResult::Error => unimplemented!(),
        _other => panic!("unhandled enum: {_other:?}"),
    }

    // tag metadata, iterate through all tags
    debug!("{orig_path}: collecting tags");
    let mut title = None;
    let mut artist = None;
    let mut artist_sort = None;
    let mut album = None;
    let mut album_sort = None;
    let mut img = None;
    let mut disk_track = (None, None);
    let mut set = HashMap::new();
    for (tag, data) in data.audio_streams().iter().flat_map(|streaminfo| {
        let tags = streaminfo.tags();
        let mut ret = vec![];
        match tags {
            Some(tags) => {
                for (tag, value) in tags.iter() {
                    trace!("{orig_path}: tag proc'd \"{tag}\"");
                    ret.push((tag.to_owned(), value));
                }
            }
            None => debug!("{orig_path}: streaminfo.tags() produced a none"),
        };
        ret
    }) {
        trace!("{orig_path}: tag \"{tag}\"");
        match tag.as_str() {
            "image" => {
                let samp = data.get::<gstreamer::Sample>()?;
                if let Some(bufs) = samp.buffer() {
                    let img_raw = {
                        let mut imgbuf = vec![];
                        for buf in bufs {
                            buf.map_readable()
                                .expect("image memory should be readable: {}")
                                .iter()
                                .for_each(|x| imgbuf.push(*x));
                        }
                        imgbuf
                    };

                    // convert to webp
                    let dropped = image::load_from_memory(&img_raw)?;
                    let conv = dropped.into_rgb8();
                    let mut hold = Cursor::new(vec![]);
                    conv.write_to(&mut hold, image::ImageFormat::WebP)?;
                    let img_buf = hold.into_inner();
                    let imghash = hash(&img_buf);
                    trace!("{orig_path}: imghash: {:?}", imghash);
                    img = Some((img_buf, imghash));
                } else {
                    anyhow::bail!("no buffer found for image");
                }
            }
            "title" => {
                title = proc_tag(data);
                trace!("{orig_path}: title is {:?}", title)
            }
            "artist" => {
                artist = proc_tag(data);
                trace!("{orig_path}: artist is {:?}", artist)
            }
            "artist-sortname" => {
                artist_sort = proc_tag(data);
                trace!("{orig_path}: artist sortname is {:?}", artist_sort);
            }
            "album" => {
                album = proc_tag(data);
                trace!("{orig_path}: album is {:?}", album)
            }
            "album-sortname" => {
                album_sort = proc_tag(data);
                trace!("{orig_path}: album sortname is {:?}", album_sort);
            }
            "album-disc-number" | "track-number" => {
                let mut_info = if tag.as_str() == "album-disc-number" {
                    &mut disk_track.0
                } else {
                    &mut disk_track.1
                };
                *mut_info = proc_tag(data).and_then(|inp| match inp.parse() {
                    Ok(ok) => Some(ok),
                    Err(err) => {
                        debug!("{orig_path}: error parsing int out {err}");
                        None
                    }
                });
                trace!("{orig_path}: disk_track is {:?}", disk_track)
            }
            _ => {
                // generic handler
                let data = proc_tag(data);
                if data.is_some() {
                    let ret = set.insert(tag.clone(), data.clone());

                    fn truncate(x: String) -> String {
                        const LEN: usize = 80;
                        if x.len() > LEN {
                            let mut trunc = x.chars().take(LEN).collect::<String>();
                            trunc.push_str("...");
                            trunc
                        } else {
                            x
                        }
                    }

                    let (data, ret) = ({ truncate(format!("{data:?}")) }, {
                        truncate(format!("{ret:?}"))
                    });
                    trace!("{orig_path}: KV inserted ({tag}, {data}), replaced ({tag}, {ret})");
                }
            }
        }
    }
    let other_tags = if !set.is_empty() {
        serde_json::to_string(
            &set.into_iter()
                .map(|(a, b)| (a.to_string(), b))
                .collect::<HashMap<_, _>>(),
        )?
    } else {
        "{}".to_string()
    };
    let title = title.unwrap_or_else(|| {
        warn!("{orig_path}: this song has no \"title\" tag, using filename");
        orig_path.to_string()
    });
    Ok(Metadata {
        title,
        other_tags,
        artist,
        artist_sort,
        album,
        album_sort,
        img,
        disk_track,
    })
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

fn proc_tag(data: SendValue) -> Option<String> {
    if let Ok(x) = data.get::<String>() {
        trace!("got string directly: {x}");
        Some(x)
    } else if let Ok(x) = data.serialize() {
        trace!("serialized string: {x}");
        Some(x.to_string())
    } else {
        trace!("no string created");
        None
    }
}

fn create_qoa(file: PathBuf, fn_dis: String) -> anyhow::Result<(mio_qoa_impl::QOADesc, Vec<i16>)> {
    use symphonia::core::errors::Error;

    info!("{fn_dis}: creating qoa encoding");
    let file = Box::new(std::fs::File::open(file)?);
    let mss = MediaSourceStream::new(file, Default::default());
    let fops: FormatOptions = Default::default();
    let mops: MetadataOptions = Default::default();
    let dops: DecoderOptions = Default::default();
    trace!("{fn_dis}: probing file");
    let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &fops, &mops)?;
    let mut fmt = probe.format;
    let track = fmt
        .default_track()
        .ok_or_else(|| anyhow!("no track found"))?;
    let track_id = track.id;
    let channel_check = track
        .codec_params
        .channels
        .ok_or_else(|| anyhow!("channels are expected in a audio file"))?;
    let channel_num = POSS_CHANNELS
        .iter()
        .find(|x| x.1 == channel_check)
        .map(|x| x.0);
    trace!("{fn_dis}: channels are {channel_num:?}");
    if channel_num.is_none() {
        anyhow::bail!("unsupported channel configuration: {channel_check}");
    }
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dops)?;
    let mut rate = None;
    let mut ret = vec![];
    loop {
        let packet = match fmt.next_packet() {
            Ok(x) => x,
            Err(Error::IoError(err)) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    trace!("{fn_dis}: finishing up");
                    break Ok((
                        mio_qoa_impl::QOADesc {
                            channels: channel_num.unwrap(),
                            sample_rate: rate.unwrap(),
                            compute_error: false,
                        },
                        ret,
                    ));
                } else {
                    return Err(Error::IoError(err).into());
                }
            }
            Err(err) => return Err(err.into()),
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buf) => {
                let spec = *buf.spec();
                let duration = symphonia::core::units::Duration::from(buf.capacity() as u64);
                let mut sample_buf = SampleBuffer::<i16>::new(duration, spec);
                if rate.is_none() {
                    trace!("{fn_dis}: rate is {}", spec.rate);
                    rate = Some(spec.rate);
                }
                if rate.is_some_and(|x| x != spec.rate) {
                    anyhow::bail!(
                        "sampling rate changed: was {rate:?}, now is {}",
                        buf.spec().rate
                    );
                }
                trace!("{fn_dis}: copying to sample_buf");
                sample_buf.copy_interleaved_ref(buf);
                trace!("{fn_dis}: copying back to ret");
                ret.extend(sample_buf.samples());
                debug!(
                    "{fn_dis}: added {} samples, sample totals is {}",
                    sample_buf.samples().len(),
                    ret.len()
                );
            }
            Err(err) => {
                return Err(anyhow::Error::from(err));
            }
        }
    }
}

fn create_vec(orig: &[i16], channels: u32, sample_rate: u32) -> anyhow::Result<Vec<f32>> {
    // pad tracks shorter than 5 seconds
    let padded = {
        let full_len = sample_rate as usize * 5;
        let mut new = orig.iter().copied().collect::<Vec<i16>>();
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
        trace!("CREATE_VEC: making mono");
        floated = floated
            .chunks(channels as usize)
            .map(|x| x.iter().sum::<f32>() / channels as f32)
            .collect();
    }

    // resample
    if sample_rate != 22050 {
        use rubato::Resampler;

        trace!("CREATE_VEC: resampling from {sample_rate} to 22050");
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
    #[allow(clippy::let_unit_value)]
    let spec = {
        use mel_spec::prelude::*;
        use mel_spec_pipeline::*;

        debug!("CREATE_VEC: making spectrogram for inference");
        let pipeline = Pipeline::new(PipelineConfig::new(
            MelConfig::new(2048, 512, 96, 22050.0),
            None,
        ));
        todo!()
    };
    todo!()
}

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
                            let (webm_blob, img_hash_hold) = metadata.img.unwrap();
                            let img_hash = img_hash_hold.as_slice();
                            sqlx::query!(
                                "INSERT INTO cover_art
                                (id, webm_blob, img_hash)
                                VALUES (?, ?, ?);",
                                id,
                                webm_blob,
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
            let other_tags = metadata.other_tags;
            sqlx::query!(
                "INSERT INTO track 
                    (id,
                    title,
                    disk, 
                    track, 
                    tags, 
                    orig_fname, 
                    album, 
                    artist, 
                    cover_art, 
                    owner,
                    path, 
                    track_vec) 
                VALUES (?,?,?,?,?,?,?,?,?,?,?,?);",
                id,
                metadata.title,
                metadata.disk_track.0,
                metadata.disk_track.1,
                other_tags,
                orig_filename,
                album_id,
                artist_id,
                cover_art_id,
                userid,
                dir,
                track_vec
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
