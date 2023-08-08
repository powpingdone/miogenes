use burn::module::Module;
use burn::nn::loss::MSELoss;
use burn::nn::{conv::*, BatchNorm, BatchNormConfig, Linear, LinearConfig, GELU};
use burn::tensor::backend::{ADBackend, Backend};
use burn::tensor::Tensor;
use burn::train::{RegressionOutput, TrainOutput, TrainStep, ValidStep};

use crate::{SAMPLE_LEN, ENC_VECTOR};
use crate::load::SampleProced;


pub struct MiniEnc<B: Backend> {
    enc: Enc<B>,
}

#[derive(Debug, Module)]
struct MiniEncTrain<B: Backend> {
    pub enc: Enc<B>,
    pub dec: Dec<B>,
}

#[derive(Debug, Module)]
pub struct Enc<B: Backend> {
    pub inp: Conv1d<B>,
    pub norm: BatchNorm<B, 1>,
    pub act: GELU,
    pub out: Linear<B>,
}

#[derive(Debug, Module)]
pub struct Dec<B: Backend> {
    pub inp: Linear<B>,
    pub out: Conv1d<B>,
}

impl<B: Backend> Enc<B> {
    fn new() -> Self {
        let inp = Conv1dConfig::new(SAMPLE_LEN, 128, 32).init();
        let norm = BatchNormConfig::new(128).init();
        let act = GELU::new();
        let out = LinearConfig::new(128, ENC_VECTOR).init();

        Self {
            inp,
            norm,
            act,
            out,
        }
    }

    pub fn forward(&self, inp: Tensor<B, 2>) -> Tensor<B, 2> {
        let [batch, len] = inp.dims();
        let inp = inp.reshape([batch, 1, len]);
        let x = self.inp.forward(inp);
        let x = self.norm.forward(x);
        let x = self.act.forward(x);

        self.out.forward(x.flatten(2, 3))
    }
}

impl<B: Backend> Dec<B> {
    fn new() -> Self {
        let inp = LinearConfig::new(ENC_VECTOR, 128).init();
        let out = Conv1dConfig::new(128, SAMPLE_LEN, 32)
            .with_padding(burn::nn::PaddingConfig1d::Same)
            .init();

        Self { inp, out }
    }

    pub fn forward(&self, inp: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.inp.forward(inp);
        let [batch, len] = x.dims();
        self.out.forward(x.reshape([batch, 1, len])).flatten(2, 3)
    }
}

impl<B: Backend> MiniEncTrain<B> {
    fn new(enc: Enc<B>, dec: Dec<B>) -> Self {
        Self { enc, dec }
    }

    pub fn forward(&self, inp: Tensor<B, 2>) -> Tensor<B, 2> {
        let x = self.enc.forward(inp);
        self.dec.forward(x)
    }

    pub fn forward_regress(&self, inp: SampleProced<B>) -> RegressionOutput<B> {
        let out = self.forward(inp.waveform_part.clone());
        let loss = MSELoss::new().forward(
            inp.waveform_part.clone(),
            out.clone(),
            burn::nn::loss::Reduction::Auto,
        );

        RegressionOutput {
            loss,
            output: out,
            targets: inp.waveform_part,
        }
    }
}

impl<B: ADBackend> TrainStep<SampleProced<B>, RegressionOutput<B>> for MiniEncTrain<B> {
    fn step(&self, item: SampleProced<B>) -> burn::train::TrainOutput<RegressionOutput<B>> {
        let item = self.forward_regress(item);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<SampleProced<B>, RegressionOutput<B>> for MiniEncTrain<B> {
    fn step(&self, item: SampleProced<B>) -> RegressionOutput<B> {
        self.forward_regress(item)
    }
}
