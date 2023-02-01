use base64::alphabet::STANDARD;
use base64::engine::{
    GeneralPurpose,
    GeneralPurposeConfig,
};
use base64::Engine;
use gloo_net::http::Request;
use mio_common::*;

// get login token
pub async fn get_token(user: String, pass: String) -> Result<msgstructs::UserToken, String> {
    let ret =
        Request::get("/l/login")
            .header(
                "Authorization",
                &format!(
                    "Basic {}",
                    GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::new()).encode(format!("{user}:{pass}"))
                ),
            )
            .send()
            .await;
    match ret {
        Ok(res) => {
            if res.ok() {
                match res.json().await {
                    Ok(ret) => Ok(ret),
                    Err(err) => Err(format!("failed to seralize: {err}")),
                }
            } else {
                Err(format!("server returned err: {}, {:?}", res.status(), res.body()))
            }
        },
        Err(err) => Err(format!("failed to connect to server: {err}")),
    }
}

pub async fn signup_send(user: String, pass: String) -> Result<(), String> {
    let ret =
        Request::post("/l/signup")
            .header(
                "Authorization",
                &format!(
                    "Basic {}",
                    GeneralPurpose::new(&STANDARD, GeneralPurposeConfig::new()).encode(format!("{user}:{pass}"))
                ),
            )
            .send()
            .await;
    match ret {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
