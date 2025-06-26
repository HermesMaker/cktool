use anyhow::{Context, Result};
use json::JsonValue;

use super::Downloader;

impl Downloader {
    /// Fetches all post attachments from a specific page URL
    ///
    /// # Arguments
    /// * `url` - The URL of the post page
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - Vector of file paths to download
    pub async fn get_posts_from_page(&self, url: &str) -> Result<Vec<String>> {
        let res = reqwest::get(url).await?;
        let text = res
            .text()
            .await
            .context("Cannot convert response body to text [res.text()]")?;
        let obj = json::parse(&text).context("Cannot parse JSON from response body")?;

        let mut posts = Vec::new();

        // Add attachments
        if let JsonValue::Array(attachments) = &obj["attachments"] {
            for atta in attachments {
                posts.push(format!("{}/data{}", atta["server"], atta["path"]));
            }
        }
        // Add previews
        if let JsonValue::Array(previews) = &obj["previews"] {
            for preview in previews {
                posts.push(format!("{}/data{}", preview["server"], preview["path"]));
            }
        }

        Ok(posts)
    }
}
