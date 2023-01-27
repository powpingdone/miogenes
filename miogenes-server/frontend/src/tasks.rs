use gloo_net::http::Request;
use oneshot as osh;

use mio_common::*;

// get login token
pub async fn get_token(
    tx: osh::Sender<Result<msgstructs::UserToken, String>>,
    user: String,
    pass: String,
) -> Result<(), String> {
    let ret = Request::get("/l/login")
        .header(
            "Authorization",
            &format!("Basic {}", base64::encode(format!("{user}:{pass}"))),
        )
        .send()
        .await;

    let ret = {
        match ret {
            Ok(res) => {
                if res.ok() {
                    let ser = res.json().await;
                    match ser {
                        Ok(ret) => Ok(ret),
                        Err(err) => Err(format!("failed to seralize: {err}")),
                    }
                } else {
                    Err(format!(
                        "server returned err: {}, {:?}",
                        res.status(),
                        res.body()
                    ))
                }
            }
            Err(err) => Err(format!("failed to connect to server: {err}")),
        }
    };
    let chk = tx.send(ret.clone());
    if ret.is_ok() && chk.is_ok() {
        Ok(())
    } else if ret.is_err() {
        Err(ret.unwrap_err())
    } else {
        Err(chk.unwrap_err().to_string())
    }
}

pub async fn signup_send(
    tx: osh::Sender<Option<u16>>,
    user: String,
    pass: String,
) -> Result<(), String> {
    let ret = Request::post("/l/signup")
        .header(
            "Authorization",
            &format!("Basic {}", base64::encode(format!("{user}:{pass}"))),
        )
        .send()
        .await;

    match ret {
        Ok(resp) => {
            if let Err(x) = tx.send(Some(resp.status())) {
                Err(x.to_string())
            } else {
                Ok(())
            }
        }
        Err(err) => {
            // we *cannot fail*, else we leak memory
            let other_err = if let Err(x) = tx.send(None) {
                " AND SENDERR ".to_owned() + &x.to_string()
            } else {
                "".to_owned()
            };
            Err(err.to_string() + &other_err)
        }
    }
}
