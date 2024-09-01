use std::env;
use std::ffi::OsString;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::io::AsyncWriteExt;

// Directory to hold the database file, user directories, and all the encoded
// audio. Must be set.
pub static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

// IP addr and port for webserver. Must be set.
pub static IP_ADDR: OnceLock<IpAddr> = OnceLock::new();
pub static PORT: OnceLock<u16> = OnceLock::new();

// enable or disable signup
pub static SIGNUP_ENABLED: OnceLock<bool> = OnceLock::new();

pub async fn init_from_env() {
    // TODO: dotenvy
    //
    // Setup and check data_dir
    DATA_DIR
        .set(
            env::var_os("DATA_DIR")
                .map(|x| x.into())
                .expect("DATA_DIR is not set. This must be set in order to run the server."),
        )
        .unwrap();
    async {
        // Remove file to set a blank state, create a file, and write out file, before
        // removing again
        let write_check = DATA_DIR.get().unwrap().join("a");

        // NOTE: doesn't really matter what remove_file does here, as this is to make sure
        // that creation works and it doesn't conflict with another file named "a"
        drop(tokio::fs::remove_file(&write_check).await);
        let mut opened = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&write_check)
            .await?;
        opened.write_all(b"a").await?;
        opened.sync_all().await?;
        drop(opened);
        tokio::fs::remove_file(write_check).await?;
        Ok::<_, anyhow::Error>(())
    }
    .await
    .expect("failed to check if DATA_DIR was actually writable: {err}");

    // ipaddr and port
    IP_ADDR
        .set(
            env::var_os("IP_ADDR")
                .map(|x| {
                    x.to_str()
                        .expect("IP_ADDR must be valid UTF-8")
                        .parse()
                        .expect("IP_ADDR is not a valid address")
                })
                .expect("IP_ADDR is not set. This must be set in order to run the server."),
        )
        .unwrap();
    PORT.set(
        env::var_os("PORT")
            .map(|x| {
                x.to_str()
                    .expect("PORT must be valid UTF-8")
                    .parse()
                    .expect("PORT is not a valid number between 0 and 65535 inclusive")
            })
            .expect("PORT is not set. This must be set in order to run the server."),
    )
    .unwrap();

    // signup enable
    SIGNUP_ENABLED
        .set(
            env::var_os("SIGNUP_ENABLED")
                .and_then(var_to_bool)
                .unwrap_or(false),
        )
        .unwrap();
}

fn var_to_bool(x: OsString) -> Option<bool> {
    x.to_str().and_then(|x| {
        if ["true", "yes", "y", "1"].contains(&x) {
            Some(true)
        } else if ["false", "no", "n", "0"].contains(&x) {
            Some(false)
        } else {
            None
        }
    })
}
