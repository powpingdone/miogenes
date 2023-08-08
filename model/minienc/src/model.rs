use burn::config::Config;
use burn::data::dataloader::DataLoaderBuilder;
use burn::module::Module;
use burn::nn::loss::MSELoss;
use burn::nn::{conv::*, BatchNorm, BatchNormConfig, Linear, LinearConfig, GELU};
use burn::optim::decay::WeightDecayConfig;
use burn::optim::AdamConfig;
use burn::record::CompactRecorder;
use burn::tensor::backend::{ADBackend, Backend};
use burn::tensor::{Data, Tensor};
use burn::train::metric::{AccuracyMetric, LossMetric};
use burn::train::{Learner, LearnerBuilder, RegressionOutput, TrainOutput, TrainStep, ValidStep};

use crate::load::SamplesTensor;
use crate::{ENC_VECTOR, SAMPLE_LEN};

const MID_LAYER: usize = 256;

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
        let inp = Conv1dConfig::new(SAMPLE_LEN, MID_LAYER, 32).init();
        let norm = BatchNormConfig::new(MID_LAYER).init();
        let act = GELU::new();
        let out = LinearConfig::new(MID_LAYER, ENC_VECTOR).init();

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
        let inp = LinearConfig::new(ENC_VECTOR, MID_LAYER).init();
        let out = Conv1dConfig::new(MID_LAYER, SAMPLE_LEN, 32)
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

    pub fn forward_regress(&self, inp: SamplesTensor<B>) -> RegressionOutput<B> {
        let out = self.forward(inp.waveforms.clone());
        let loss = MSELoss::new().forward(
            inp.waveforms.clone(),
            out.clone(),
            burn::nn::loss::Reduction::Auto,
        );

        RegressionOutput {
            loss,
            output: out,
            targets: inp.waveforms,
        }
    }
}

impl<B: ADBackend> TrainStep<SamplesTensor<B>, RegressionOutput<B>> for MiniEncTrain<B> {
    fn step(&self, item: SamplesTensor<B>) -> burn::train::TrainOutput<RegressionOutput<B>> {
        let item = self.forward_regress(item);
        TrainOutput::new(self, item.loss.backward(), item)
    }
}

impl<B: Backend> ValidStep<SamplesTensor<B>, RegressionOutput<B>> for MiniEncTrain<B> {
    fn step(&self, item: SamplesTensor<B>) -> RegressionOutput<B> {
        self.forward_regress(item)
    }
}

#[derive(Config)]
pub(crate) struct TCon {
    #[config(default = 10)]
    pub epochs: usize,

    #[config(default = 64)]
    pub batch_size: usize,

    #[config(default = 4)]
    pub workers: usize,

    #[config(default = 42)]
    pub seed: u64,

    pub optimizer: AdamConfig,
}

pub(crate) fn train_run<B: ADBackend>(device: B::Device) {
    let opt = AdamConfig::new().with_weight_decay(Some(WeightDecayConfig::new(1e-7)));
    let conf = TCon::new(opt);
    B::seed(conf.seed);

    let (train_set, test_set) = crate::load::LoadSamples::new();
    let bat_train = crate::load::BatchDevice::<B>::new(device.clone());
    let bat_test = crate::load::BatchDevice::<B::InnerBackend>::new(device.clone());
    let dl_train = DataLoaderBuilder::new(bat_train)
        .batch_size(conf.batch_size)
        .shuffle(conf.seed)
        .num_workers(conf.workers)
        .build(train_set);
    let dl_test = DataLoaderBuilder::new(bat_test)
        .batch_size(conf.batch_size)
        .shuffle(conf.seed)
        .num_workers(conf.workers)
        .build(test_set);
    let learner = LearnerBuilder::<B, _, _, MiniEncTrain<B>, _, f64>::new("./artifact/")
        .metric_train_plot(AccuracyMetric::new())
        .metric_valid_plot(AccuracyMetric::new())
        .metric_train_plot(LossMetric::new())
        .metric_valid_plot(LossMetric::new())
        .with_file_checkpointer(1, CompactRecorder::new())
        .devices(vec![device])
        .num_epochs(conf.epochs)
        .build(
            MiniEncTrain::new(Enc::<B>::new(), Dec::new()),
            conf.optimizer.init(),
            1e-4,
        );
}
