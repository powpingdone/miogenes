use super::{CurrentlyDecoding, DecoderMsg};
use crate::player::decoder::ControllingDecoder;
use crate::*;
use log::*;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub struct Player {
    pub tx: crossbeam::channel::Sender<DecoderMsg>,
    pub rx: tokio::sync::watch::Receiver<CurrentlyDecoding>,
    _dev: rodio::OutputStream,
    _s_handle: rodio::OutputStreamHandle,
    _dec_thread: std::thread::JoinHandle<()>,
}

impl Player {
    pub fn new(client: Arc<RwLock<MioClientState>>) -> anyhow::Result<Self> {
        let (tx_player, rx_player) = crossbeam::channel::unbounded();
        let (tx_pstate, rx_pstate) = tokio::sync::watch::channel(CurrentlyDecoding {
            tracks: vec![],
            curr: Uuid::nil(),
            at: Duration::new(0, 0),
            len: Duration::new(0, 0),
        });

        trace!("acqiring dev");
        let (_dev, s_handle) = find_dev()?;
        trace!("setting up decoder");
        let decoder = ControllingDecoder::new(client, tx_pstate, rx_player);

        Ok(Self {
            tx: tx_player,
            rx: rx_pstate,
            // task does not get joined due to if tx_player gets dropped, then everything
            // else will die as well
            _dec_thread: std::thread::spawn({
                let s_handle = s_handle.clone();
                move || {
                    trace!("spinning s_thread");
                    s_handle.play_raw(decoder).unwrap();
                }
            })
            .into(),
            _dev: _dev.into(),
            _s_handle: s_handle,
        })
    }
}

fn find_dev() -> anyhow::Result<(rodio::OutputStream, rodio::OutputStreamHandle)> {
    #[cfg(not(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    )))]
    {
        use std::panic::catch_unwind;

        debug!("not on linux: attempting to get default device");
        Ok({
            let x = catch_unwind(rodio::OutputStream::try_default);
            if let Err(ref err) = x {
                return Err(anyhow::anyhow!("panicked: {:?}", {
                    if let Some(x) = err.downcast_ref::<&str>() {
                        Some(*x)
                    } else {
                        err.downcast_ref::<String>().map(|x| x.as_str())
                    }
                }));
            }
            x.unwrap()
        }?)
    }

    // select jack by default on everything that _can_ use alsa. alsa sucks.
    #[cfg(any(
        target_os = "linux",
        target_os = "dragonfly",
        target_os = "freebsd",
        target_os = "netbsd"
    ))]
    {
        use rodio::cpal::traits::HostTrait;

        trace!("on \"linux\": trying to get jack");
        rodio::OutputStream::try_from_device(
            &cpal::host_from_id(
                cpal::available_hosts()
                    .into_iter()
                    .find(|x| *x == cpal::HostId::Jack)
                    .ok_or(anyhow::anyhow!("No jack host found"))?,
            )?
            .default_output_device()
            .ok_or(anyhow::anyhow!(
                "jack host found but no default output device found"
            ))?,
        )
        .map_err(|err| err.into())
    }
}
