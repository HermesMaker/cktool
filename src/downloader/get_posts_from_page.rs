use std::{
    thread::{self},
    time::Duration,
};

use anyhow::{Context, Result};
use json::JsonValue;

use crate::declare;

use super::Downloader;

impl Downloader {
    /// Fetches all post attachments from a specific page URL
    ///
    /// # Arguments
    /// * `url` - The URL of the post page
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - Vector of file paths to download
    pub async fn get_posts_from_page(&mut self, url: &str) -> Result<Vec<String>> {
        let mut posts = Vec::new();
        let mut json_parse_retry = self.retry;
        let mut http_retry = self.retry;
        loop {
            let res = match reqwest::get(url).await {
                Ok(v) => v,
                Err(_) => {
                    if !http_retry == 0 {
                        http_retry -= 1;
                        thread::sleep(Duration::from_secs(declare::ERROR_REQUEST_DELAY_SEC));
                        continue;
                    }
                    return Err(anyhow::anyhow!(
                        "Failed http request in `get_posts_from_page`"
                    ));
                }
            };

            let text = res
                .text()
                .await
                .context("Cannot convert response body to text [res.text()]")?;
            let obj = match json::parse(&text).context("Cannot parse JSON from response body") {
                Ok(v) => v,
                Err(_) => {
                    if json_parse_retry > 0 {
                        json_parse_retry -= 1;
                        thread::sleep(Duration::from_secs(declare::TOO_MANY_REQUESTS_DELAY_SEC));
                        continue;
                    } else {
                        break;
                    }
                }
            };

            let mut is_skip = false;
            // Add attachments
            if let JsonValue::Array(attachments) = &obj["attachments"] {
                for atta in attachments {
                    if atta["server"].is_null() || atta["path"].is_null() {
                        is_skip = true;
                    } else {
                        posts.push(format!("{}/data{}", atta["server"], atta["path"]));
                    }
                }
            }
            // Add previews
            if let JsonValue::Array(previews) = &obj["previews"] {
                for preview in previews {
                    // Some of videos could not be download, so it will be skipped.
                    if preview["server"].is_null() || preview["path"].is_null() {
                        if is_skip {
                            self.info.lock().await.add_skip_file(url.to_string());
                        }
                    } else {
                        posts.push(format!("{}/data{}", preview["server"], preview["path"]));
                    }
                }
            }
            break;
        }
        Ok(posts)
    }
}
