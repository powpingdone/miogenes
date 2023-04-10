use mio_common::*;
use reqwest::{
    Client,
    StatusCode,
};
use crate::BASE_URL;

// get login token
pub async fn get_token(user: String, pass: String) -> Result<msgstructs::UserToken, String> {
    let client = Client::new();
    let ret = client.get(format!("{}/l/login", BASE_URL.get().unwrap())).basic_auth(user, Some(pass)).send().await;
    match ret {
        Ok(res) => {
            if res.status() == StatusCode::OK {
                match res.json().await {
                    Ok(ret) => Ok(ret),
                    Err(err) => Err(format!("failed to seralize: {err}")),
                }
            } else {
                Err(format!("server returned err: {}, {:?}", res.status(), res.text().await))
            }
        },
        Err(err) => Err(format!("failed to connect to server: {err}")),
    }
}

pub async fn signup_send(user: String, pass: String) -> Result<(), String> {
    let client = Client::new();
    let ret = client.post(format!("{}/l/signup", BASE_URL.get().unwrap())).basic_auth(user, Some(pass)).send().await;
    match ret {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}
