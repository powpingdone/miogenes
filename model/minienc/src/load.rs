use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use burn::{
    data::dataset::Dataset,
    tensor::{backend::Backend, Float, Tensor},
};

struct LoadSamples {
    paths: Vec<PathBuf>,
}

impl LoadSamples {
    pub(crate) fn new(audio_dir: impl AsRef<Path>) -> Self {
        Self {
            paths: read_dir(audio_dir)
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
                .collect(),
        }
    }
}

impl<B: Backend> Dataset<SampleProced<B>> for LoadSamples {
    fn get(&self, index: usize) -> Option<SampleProced<B>> {
        todo!()
    }

    fn len(&self) -> usize {
        println!("calculating length of all possible samples");
        todo!()
    }
}

#[derive(Debug, Clone)]
struct BatchDevice<B: Backend> {
    dev: B::Device,
}

impl<B: Backend> BatchDevice<B> {
    pub(crate) fn new(dev: B::Device) -> Self {
        Self { dev }
    }
}

pub(crate) struct SampleProced<B: Backend> {
    pub(crate) waveform_part: Tensor<B, 2, Float>,
}
