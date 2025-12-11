use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};

pub fn new() -> anyhow::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert("Accept", HeaderValue::from_static("text/css"));
    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .build()?)
}
