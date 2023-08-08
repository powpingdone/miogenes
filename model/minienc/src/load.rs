use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    vec,
};

use burn::{
    data::{dataloader::batcher::Batcher, dataset::Dataset},
    tensor::{backend::Backend, ops::TensorOps, Data, Float, Tensor},
};

pub(crate) struct LoadSamples {
    paths: Vec<PathBuf>,
}

impl LoadSamples {
    // train, test
    pub(crate) fn new() -> (Self, Self) {
        let files = read_dir("./raw/")
            .unwrap()
            .filter_map(|x| {
                let x = x.unwrap();
                if x.file_type().unwrap().is_file()
                    && AsRef::<Path>::as_ref(&x.file_name())
                        .extension()
                        .is_some_and(|x| x == "sblb")
                {
                    Some(x.file_name().into())
                } else {
                    None
                }
            })
            .collect::<Vec<PathBuf>>();
        let split_pt = (files.len() as f32 * 0.9).ceil() as usize;
        (
            Self {
                paths: files[..split_pt].to_vec(),
            },
            Self {
                paths: files[split_pt..].to_vec(),
            },
        )
    }
}

impl Dataset<SampleProced> for LoadSamples {
    fn get(&self, index: usize) -> Option<SampleProced> {
        if index > self.paths.len() {
            return None;
        }

        let x = std::fs::read(&self.paths[index])
            .unwrap()
            .chunks(4)
            .map(|float| f32::from_le_bytes(float.try_into().unwrap()))
            .collect::<Vec<_>>();

        Some(SampleProced::new(x))
    }

    fn len(&self) -> usize {
        self.paths.len()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SamplesTensor<B: Backend> {
    pub(crate) waveforms: Tensor<B, 2, Float>,
}

#[derive(Debug, Clone)]
pub(crate) struct BatchDevice<B: Backend> {
    dev: B::Device,
}

impl<B: Backend> BatchDevice<B> {
    pub(crate) fn new(dev: B::Device) -> Self {
        Self { dev }
    }
}

impl<B: Backend> Batcher<SampleProced, SamplesTensor<B>> for BatchDevice<B> {
    fn batch(&self, items: Vec<SampleProced>) -> SamplesTensor<B> {
        let dims = [items.len(), crate::SAMPLE_LEN];
        let datas = items
            .into_iter()
            .map(|x| Data::<f32, 1>::from(*x.waveform_part))
            .map(|x| Tensor::<B, 1>::from_data(x.convert()).reshape([1, dims[1]]))
            .collect();
        SamplesTensor {
            waveforms: Tensor::<B, 2, Float>::cat(datas, 0).to_device(&self.dev),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SampleProced {
    pub(crate) waveform_part: Box<[f32; crate::SAMPLE_LEN]>,
}

impl SampleProced {
    pub fn new(vector: Vec<f32>) -> Self {
        if vector.len() != crate::SAMPLE_LEN {
            panic!(
                "input vector length must be {}, got {} instead",
                crate::SAMPLE_LEN,
                vector.len()
            )
        }
        let mut waveform_part = Box::new([0.0; crate::SAMPLE_LEN]);
        for (x, y) in waveform_part.iter_mut().zip(vector.into_iter()) {
            *x = y;
        }
        Self { waveform_part }
    }
}
