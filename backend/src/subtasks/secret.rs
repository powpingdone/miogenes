#[allow(unused)]
use log::*;
use once_cell::sync::OnceCell;
use rand::RngCore;
use std::{io::Cursor, path::PathBuf, sync::Arc, time::SystemTime};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    sync::RwLock,
};

#[derive(Clone)]
pub struct SecretHolder {
    curr_secret: Arc<RwLock<[u8; SECRET_SIZE]>>,
}

impl std::fmt::Debug for SecretHolder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecretHolder").finish()
    }
}

const SECRET_SIZE: usize = 1024;
static REFRESH_TASK: OnceCell<tokio::task::JoinHandle<()>> = OnceCell::new();

impl SecretHolder {
    pub async fn new() -> Self {
        let mut read = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(
                [crate::DATA_DIR.get().unwrap(), "secret"]
                    .into_iter()
                    .collect::<PathBuf>(),
            )
            .await
            .expect("Failed to open secret file");
        let mut secret: [u8; SECRET_SIZE] = [0; SECRET_SIZE];
        let tryread = read.read_exact(secret.as_mut()).await;
        let secret = {
            if tryread
                .as_ref()
                .is_err_and(|err| err.kind() == std::io::ErrorKind::UnexpectedEof)
                || tryread.as_ref().is_ok_and(|len| *len != SECRET_SIZE)
            {
                new_secret(read).await
            } else if tryread.is_ok() {
                secret
            } else {
                panic!("error reading secret: {}", tryread.unwrap_err())
            }
        };
        let secret = Arc::new(RwLock::const_new(secret));
        REFRESH_TASK
            .set(tokio::spawn({
                let secret = secret.clone();
                async move {
                    loop {
                        tokio::time::sleep(stat_secret().await).await;
                        let mut write = secret.write().await;
                        *write = new_secret(
                            OpenOptions::new()
                                .write(true)
                                .open(format!("{}/secret", crate::DATA_DIR.get().unwrap()))
                                .await
                                .expect("failed to open secret for writing"),
                        )
                        .await;
                    }
                }
            }))
            .expect("only one SecretHolder may exist at the time of the Miogenes server");
        Self {
            curr_secret: secret,
        }
    }

    pub async fn get_secret(&self) -> [u8; SECRET_SIZE] {
        *self.curr_secret.read().await
    }
}

pub async fn stat_secret() -> std::time::Duration {
    let file = OpenOptions::new()
        .read(true)
        .open(format!("{}/secret", crate::DATA_DIR.get().unwrap()))
        .await
        .expect("failed to open file for metadata statting: {}");
    let dur = SystemTime::now()
        .duration_since(file.metadata().await.unwrap().modified().unwrap())
        .unwrap_or(std::time::Duration::ZERO);
    if dur
        > std::time::Duration::from_secs(
            // seconds in a week
            604800,
        )
    {
        std::time::Duration::ZERO
    } else {
        dur
    }
}

async fn new_secret(mut file: File) -> [u8; SECRET_SIZE] {
    let mut new = [0; SECRET_SIZE];
    rand::thread_rng().fill_bytes(new.as_mut());
    file.set_len(0).await.unwrap();
    let mut copy_cursor = Cursor::new(new);
    file.write_buf(&mut copy_cursor).await.unwrap();
    file.sync_all().await.unwrap();
    new
}
