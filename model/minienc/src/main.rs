use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

const SAMPLE_LEN: usize = 1050;
const ENC_VECTOR: usize = 16;

mod load;
mod model;

fn make_blobs() {
    for file in read_dir("./audio/").unwrap() {
        let file = file.unwrap();
        if file.file_type().unwrap().is_file() {
            let fname_hold = file.file_name();
            let fname = fname_hold.to_string_lossy();
            println!("processing {fname}");
            match blob_get(&file.path()) {
                Ok((mut vec, rate, channels)) => {

                    // make mono
                    if channels != 1 {
                        println!("converting {fname} to mono");
                        let mut new_vec = Vec::with_capacity(vec.len() / channels);
                        for pos in 0..vec.len() / channels {
                            new_vec.push(
                                vec[pos * channels..(pos + 1) * channels]
                                    .iter()
                                    .sum::<f32>()
                                    / channels as f32,
                            );
                        }
                        vec = new_vec;
                    }

                    // change sample rate
                    if rate != 44100 {
                        use rubato::Resampler;
                        
                        println!("resampling {fname} from {rate} to 44100");
                        let mut new_vec = Vec::<f32>::new();
                        let mut resamp =
                            rubato::FftFixedIn::<f32>::new(rate as usize, 44100, 1024, 2, 1)
                                .unwrap();
                        let mut out_buf = vec![vec![0.0f32; resamp.output_frames_max()]; 1];
                        let mut old_vec = vec![&vec[..]];
                        while old_vec.len() >= resamp.input_frames_next() {
                            let (lin, lout) = resamp
                                .process_into_buffer(&old_vec, &mut out_buf, None)
                                .unwrap();
                            old_vec[0] = &old_vec[0][lin..];
                            new_vec.extend_from_slice(&out_buf[0][..lout]);
                        }

                        if old_vec[0].is_empty() {
                            let (_, lout) = resamp
                                .process_partial_into_buffer(Some(&old_vec), &mut out_buf, None)
                                .unwrap();
                            new_vec.extend_from_slice(&out_buf[0][..lout]);
                        }
                        vec = new_vec;
                    }

                    if vec.is_empty() {
                        // TODO: figure out what's causing this
                        println!("somehow, file is empty, skipping");
                        continue;
                    }
                    
                    // write out
                    //
                    // tmp solution to not spam files
                    let third_len = vec.len() / 3;
                    let range = third_len..(third_len + (30 * 44100)).clamp(0, vec.len() - 1);
                    for x in vec[range].chunks(SAMPLE_LEN) {
                        if x.len() == SAMPLE_LEN {
                            let name = ["raw", &format!("{}.sblb", uuid::Uuid::new_v4())]
                                .into_iter()
                                .collect::<PathBuf>();
                            std::fs::write(
                                name,
                                x.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>(),
                            )
                            .unwrap();
                        }
                    }
                }
                Err(err) => {
                    println!("failed to process file {}: {err}", file.path().display());
                    continue;
                }
            }
        }
    }
}

fn blob_get(file: impl AsRef<Path>) -> anyhow::Result<(Vec<f32>, u32, usize)> {
    use symphonia::core::{
        audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
        io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
    };

    let file = Box::new(std::fs::File::open(file)?);
    let mss = MediaSourceStream::new(file, Default::default());
    let fops: FormatOptions = Default::default();
    let mops: MetadataOptions = Default::default();
    let dops: DecoderOptions = Default::default();
    let probe = symphonia::default::get_probe().format(&Hint::new(), mss, &fops, &mops)?;
    let mut fmt = probe.format;
    let track = fmt.default_track().unwrap();
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dops)?;
    let mut rate = None;
    let mut ret = vec![];
    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| anyhow::anyhow!("channels are expected in a audio file"))?
        .count();
    loop {
        let packet = match fmt.next_packet() {
            Ok(x) => x,
            Err(Error::IoError(err)) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    break Ok((ret, rate.unwrap(), channels));
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
                let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
                if rate.is_none() {
                    rate = Some(spec.rate);
                }
                if rate.is_some_and(|x| x != spec.rate) {
                    anyhow::bail!(
                        "sampling rate changed: was {rate:?}, now is {}",
                        buf.spec().rate
                    );
                }
                sample_buf.copy_interleaved_ref(buf);
                ret.extend(sample_buf.samples());
            }
            Err(err) => {
                return Err(anyhow::Error::from(err));
            }
        }
    }
}

fn main() {
    let action = std::env::args()
        .nth(1)
        .expect("expected an action of 'preproc', 'train'");
    match action.as_str() {
        "preproc" => make_blobs(),
        // TODO: build miniencoder burn model
        "train" => todo!(),
        _a => panic!("action {_a} is not a valid action"),
    }
}
