use bytemuck::NoUninit;
use thiserror::Error;

// Most of this was directly extracted from the qoa.h file in the qoa repository.
// As such, this crate is not licensed under GPL, but MIT, and also all rights go
// to phoboslab for what will happen.
const QOA_MAX_CHANNELS: u32 = 8;
const QOA_SLICE_LEN: usize = 20;
const QOA_SLICES_PER_FRAME: usize = 256;
const QOA_FRAME_LEN: usize = QOA_SLICES_PER_FRAME * QOA_SLICE_LEN;
const QOA_LMS_LEN: usize = 4;
const QOA_MAGIC: u32 = 0x716f6166;

// The quant_tab provides an index into the dequant_tab for residuals in the range
// of -8 .. 8. It maps this range to just 3bits and becomes less accurate at the
// higher end. Note that the residual zero is identical to the lowest positive
// value. This is mostly fine, since the qoa_div() function always rounds away
// from zero.
const QOA_QUANT_TAB: [usize; 17] = [7, 7, 7, 5, 5, 3, 3, 1, 0, 0, 2, 2, 4, 4, 6, 6, 6];

// The reciprocal_tab maps each of the 16 scalefactors to their rounded
// reciprocals 1/scalefactor. This allows us to calculate the scaled residuals in
// the encoder with just one multiplication instead of an expensive division. We
// do this in .16 fixed point with integers, instead of floats.
//
// The reciprocal_tab is computed as: reciprocal_tab[s] <- ((1<<16) +
// scalefactor_tab[s] - 1) / scalefactor_tab[s]
const QOA_RECIPROCAL_TAB: [i64; 16] = [
    65536, 9363, 3121, 1457, 781, 475, 311, 216, 156, 117, 90, 71, 57, 47, 39, 32,
];

// The dequant_tab maps each of the scalefactors and quantized residuals to their
// unscaled & dequantized version.
//
// Since qoa_div rounds away from the zero, the smallest entries are mapped to 3/4
// instead of 1. The dequant_tab assumes the following dequantized values for each
// of the quant_tab indices and is computed as: float dqt[8] = {0.75, -0.75, 2.5,
// -2.5, 4.5, -4.5, 7, -7}; dequant_tab[s][q] <-
// round_ties_away_from_zero(scalefactor_tab[s] * dqt[q])
//
// The rounding employed here is "to nearest, ties away from zero",  i.e. positive
// and negative values are treated symmetrically.
const QOA_DEQUANT_TAB: [[i32; 8]; 16] = [
    [1, -1, 3, -3, 5, -5, 7, -7],
    [5, -5, 18, -18, 32, -32, 49, -49],
    [16, -16, 53, -53, 95, -95, 147, -147],
    [34, -34, 113, -113, 203, -203, 315, -315],
    [63, -63, 210, -210, 378, -378, 588, -588],
    [104, -104, 345, -345, 621, -621, 966, -966],
    [158, -158, 528, -528, 950, -950, 1477, -1477],
    [228, -228, 760, -760, 1368, -1368, 2128, -2128],
    [316, -316, 1053, -1053, 1895, -1895, 2947, -2947],
    [422, -422, 1405, -1405, 2529, -2529, 3934, -3934],
    [548, -548, 1828, -1828, 3290, -3290, 5117, -5117],
    [696, -696, 2320, -2320, 4176, -4176, 6496, -6496],
    [868, -868, 2893, -2893, 5207, -5207, 8099, -8099],
    [1064, -1064, 3548, -3548, 6386, -6386, 9933, -9933],
    [1286, -1286, 4288, -4288, 7718, -7718, 12005, -12005],
    [1536, -1536, 5120, -5120, 9216, -9216, 14336, -14336],
];

#[derive(Debug, Clone)]
struct LMSFilter {
    // noted in paper as "Sign-Sign Least Mean Squares Filter"
    pub(crate) history: [i32; QOA_LMS_LEN],
    pub(crate) weights: [i32; QOA_LMS_LEN],
}

impl Default for LMSFilter {
    fn default() -> Self {
        Self {
            history: [0, 0, 0, 0],
            // weights init is wrong. the comment says {0,0,-1,2}, but it's actually
            weights: [0, 0, -8192, 16384],
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct QOADesc {
    // external selection
    pub channels: u32,
    pub sample_rate: u32,
    // benchmarking, questioning
    pub compute_error: bool,
}

#[derive(Debug, Clone)]
pub struct QOAEncoded {
    pub raw_file: Vec<u8>,
    pub error: Option<u64>,
}

impl QOAEncoded {
    #[inline]
    fn write_into<T>(&mut self, x: T)
    where
        T: NoUninit,
    {
        let bytes = bytemuck::bytes_of(&x);
        for pos in 0..bytes.len() {
            self.raw_file.push(if cfg!(target_endian = "little") {
                bytes[bytes.len() - 1 - pos]
            } else {
                bytes[pos]
            });
        }
    }
}

#[derive(Debug, Error)]
pub enum QOAError {
    #[error("invalid options given to encoder: {0} ")]
    InvalidOpts(&'static str),
}

// Waveform is encoded as raw signed 16 bit pcm data, where channels interleave
// each u16 per sample
#[inline(never)]
pub fn encode(waveform: impl AsRef<[i16]>, desc: &QOADesc) -> Result<QOAEncoded, QOAError> {
    let waveform = waveform.as_ref();
    // prelude to make sure that the description is good
    if waveform.is_empty() || waveform.len() > u32::MAX as usize {
        return Err(QOAError::InvalidOpts(
            "waveform length must be between 0 and u32::MAX",
        ));
    }
    if waveform.len() % desc.channels as usize != 0 {
        return Err(QOAError::InvalidOpts(
            "waveform length must be a multiple of the channels",
        ));
    }
    if !(0 < desc.sample_rate && desc.sample_rate <= 0xFFFFFF) {
        return Err(QOAError::InvalidOpts(
            "sample rate must be between 0 and 0xFFFFFF",
        ));
    }
    if !(0 < desc.channels && desc.channels <= QOA_MAX_CHANNELS) {
        return Err(QOAError::InvalidOpts("channels must be between 0 and 9"));
    }

    // setup return type
    let u_channels = desc.channels as usize;
    let mut ret = QOAEncoded {
        error: if desc.compute_error { Some(0) } else { None },
        raw_file: Vec::with_capacity({
            let frames = (waveform.len() + QOA_FRAME_LEN - 1) / QOA_FRAME_LEN;
            let slices = (waveform.len() + QOA_SLICE_LEN - 1) / QOA_SLICE_LEN;
            8 + frames * 8 + frames * QOA_LMS_LEN * 4 * u_channels + slices * 8 * u_channels
        }),
    };

    // write header
    ret.write_into(QOA_MAGIC);
    ret.write_into::<u32>(waveform.len() as u32 / desc.channels);

    // get number of vec slices needed
    let slice_incs = if waveform.len() % QOA_FRAME_LEN != 0 {
        (waveform.len() / (QOA_FRAME_LEN * desc.channels as usize)) + 1
    } else {
        waveform.len() / (QOA_FRAME_LEN * desc.channels as usize)
    };

    // write frames
    let mut lms_channel = vec![LMSFilter::default(); u_channels];
    for slice in (0..slice_incs).map(|x| {
        let sample_index = x * QOA_FRAME_LEN;
        &waveform[(sample_index * u_channels)
            ..((sample_index + QOA_FRAME_LEN) * u_channels + u_channels - 1)
                .clamp(0, waveform.len())]
    }) {
        let frame_len = slice.len() / u_channels;
        let frame_size = (8
            + QOA_LMS_LEN * 4 * u_channels
            + 8 * (((frame_len) + QOA_SLICE_LEN - 1) / QOA_SLICE_LEN) * u_channels)
            as u64;

        // write frame header
        //
        // setup is u8, u24, u16, u16
        ret.write_into(
            (desc.channels as u64) << 56
                | (desc.sample_rate as u64) << 32
                | (frame_len as u64) << 16
                | frame_size,
        );

        // reset weights if too large, see https://github.com/phoboslab/qoa/issues/25
        for weights in lms_channel.iter_mut().map(|x| &mut x.weights) {
            if weights.iter().map(|x| x.pow(2)).sum::<i32>() > 0x2fffffff {
                weights.iter_mut().for_each(|x| *x = 0);
            }
        }

        // write history then weights
        for lms in lms_channel.iter() {
            lms.history
                .iter()
                .chain(lms.weights.iter())
                .for_each(|x| ret.write_into((*x & 0xffff) as u16))
        }

        // get true SOA slices
        let mut prev_scalefactor = vec![0u64; u_channels];
        for (slice_len, soa_slices) in (0..slice.len() - (u_channels - 1))
            .step_by(QOA_SLICE_LEN * u_channels)
            .map(|offset| {
                (
                    (offset + QOA_SLICE_LEN * u_channels).clamp(0, slice.len()) - offset,
                    (0..u_channels)
                        .map(|c| {
                            &slice[offset + c
                                ..(offset + c + QOA_SLICE_LEN * u_channels).clamp(0, slice.len())]
                        })
                        .collect::<Vec<_>>(),
                )
            })
        {
            for (channel, soa_slice) in soa_slices.into_iter().enumerate() {
                let channel = channel % u_channels;
                let mut best_error = u64::MAX;
                let mut best_slice = 0u64;
                let mut best_lms = LMSFilter::default();
                let mut best_scalefactor = 0u64;

                // bruteforce all 16 slices
                for sfi in 0u64..16 {
                    // test best scalefactor of previous slice first
                    let scalefactor = (sfi + prev_scalefactor[channel]) % 16;
                    let mut lms = lms_channel[channel].clone();
                    let mut slice = scalefactor;
                    let mut curr_error = 0u64;
                    for sample in soa_slice.iter().step_by(u_channels).map(|x| *x as i32) {
                        // qoa_lms_predict
                        let pred = lms
                            .history
                            .iter()
                            .zip(lms.weights.iter())
                            .map(|(h, w)| h * w)
                            .sum::<i32>()
                            >> 13;
                        let residual = sample - pred;

                        // qoa_div
                        let scaled = {
                            let right = QOA_RECIPROCAL_TAB[scalefactor as usize];
                            let n = (residual as i64 * right + (1 << 15)) >> 16;
                            n + ((residual > 0) as i64 - (residual < 0) as i64)
                                - ((n > 0) as i64 - (n < 0) as i64)
                        };
                        let clamped = scaled.clamp(-8, 8);
                        let quantized = QOA_QUANT_TAB[(clamped + 8) as usize];
                        let dequantized = QOA_DEQUANT_TAB[scalefactor as usize][quantized];
                        let reconstructed =
                            (pred + dequantized).clamp(i16::MIN as i32, i16::MAX as i32);

                        // break on worse error
                        let err = (sample - reconstructed) as i64;
                        curr_error += err.pow(2) as u64;
                        if curr_error > best_error {
                            break;
                        }

                        // update lms and curr slice
                        let delta = dequantized >> 4;
                        lms.weights
                            .iter_mut()
                            .zip(lms.history.iter())
                            .for_each(|(weig, hist)| {
                                *weig += if *hist < 0 { -delta } else { delta }
                            });
                        (0..QOA_LMS_LEN - 1).for_each(|x| lms.history[x] = lms.history[x + 1]);
                        lms.history[QOA_LMS_LEN - 1] = reconstructed;
                        slice = (slice << 3) | quantized as u64;
                    }
                    if curr_error < best_error {
                        best_error = curr_error;
                        best_slice = slice;
                        best_lms = lms;
                        best_scalefactor = scalefactor;
                    }
                }

                // finalize slice
                prev_scalefactor[channel] = best_scalefactor;
                lms_channel[channel] = best_lms.clone();
                if let Some(ref mut x) = ret.error {
                    *x += best_error;
                }

                // if short, shift bytes
                best_slice <<= (QOA_SLICE_LEN - slice_len / u_channels) * 3;
                ret.write_into(best_slice);
            }
        }
    }
    Ok(ret)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        ffi::OsStr,
        fs::{read, read_dir},
        path::PathBuf,
    };

    #[test]
    fn functional() {
        super::encode(
            vec![0i16; 644706],
            &QOADesc {
                channels: 2,
                sample_rate: 44100,
                compute_error: true,
            },
        )
        .unwrap();
    }

    #[test]
    #[ignore]
    fn verify_against_suite() {
        use hound::WavReader;

        for x in read_dir("./wav").unwrap() {
            let x = x.unwrap().path();
            let path_disp_inner = x.clone();
            let path_disp = path_disp_inner.display();
            println!("trying {}", path_disp);
            let mut wav = WavReader::open(x.clone()).unwrap();
            let verify = std::thread::spawn(move || {
                let mut fname = x.file_stem().unwrap().to_owned();
                fname.push(".qoa");
                let path = [OsStr::new("./qoa"), &fname]
                    .into_iter()
                    .collect::<PathBuf>();
                read(path).unwrap()
            });
            let enc = super::encode(
                wav.samples().map(|x| x.unwrap()).collect::<Vec<i16>>(),
                &QOADesc {
                    channels: wav.spec().channels as u32,
                    sample_rate: wav.spec().sample_rate,
                    compute_error: false,
                },
            )
            .unwrap();
            let verify = verify.join().unwrap();
            assert_eq!(verify.len(), enc.raw_file.len());
            for ((i, gen), actual) in enc.raw_file.iter().enumerate().zip(&verify) {
                // on error
                if *gen != *actual {
                    let err_range =
                        i.saturating_sub(QOA_SLICE_LEN)..(i + QOA_SLICE_LEN).clamp(0, verify.len());
                    let fmt = format!(
                        "{}, byte {i} does not match. expected {actual:02x} got {gen:02x}.",
                        path_disp
                    );
                    let fmt = format!("{fmt}\n\tsurrounding bytes:\n");
                    let here = format!(
                        "{}{}{}",
                        "        ".to_owned(),
                        if i < 20 {
                            (0..(i.saturating_sub(QOA_SLICE_LEN)).clamp(i, QOA_SLICE_LEN))
                                .map(|_| "   ")
                                .collect::<String>()
                        } else {
                            (0..QOA_SLICE_LEN).map(|_| "   ").collect::<String>()
                        },
                        "**"
                    );
                    panic!(
                        "{fmt}{here}\nactual: {}\nencode: {}",
                        &verify[err_range.clone()]
                            .iter()
                            .map(|x| format!("{x:02x} "))
                            .collect::<String>(),
                        &enc.raw_file[err_range]
                            .iter()
                            .map(|x| format!("{x:02x} "))
                            .collect::<String>()
                    );
                }
            }
        }
    }
}
