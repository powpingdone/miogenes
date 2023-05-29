use once_cell::sync::Lazy;

pub(crate) static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());
