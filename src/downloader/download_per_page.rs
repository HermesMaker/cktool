use anyhow::{Context, Result};
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use std::cmp::min;
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufWriter},
    time::{Duration, sleep},
};

use crate::declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC};

use super::{Downloader, page_status::PageStatus};

impl Downloader {
    /// Downloads all files from a specific page
    ///
    /// # Arguments
    /// * `pid` - post id to download
    /// * `page` - Current page information for progress display
    pub async fn download_per_page(&self, pid: String, page: PageStatus) -> Result<()> {
        let url = self.link.post_id(&pid);
        let posts = self.get_posts_from_page(&url).await?;

        for path in posts {
            let outdir = self.outdir.clone();
            let mc = self.multi_progress.clone();
            let fname = path.split("/").last().context("Invalid file path")?;

            let client = reqwest::Client::new();
            let file = File::create(format!("{}/{}", outdir, fname)).await?;
            let mut file = BufWriter::new(file);

            let mut retry = self.retry;
            loop {
                if let Ok(res) = client.get(&path).send().await {
                    let total_size = res.content_length().context("Cannot get total size")?;
                    // prevent too many requests
                    if StatusCode::TOO_MANY_REQUESTS == res.status() {
                        tokio::time::sleep(Duration::from_secs(TOO_MANY_REQUESTS_DELAY_SEC)).await;
                        continue;
                    }
                    // prevent bad gateway: wait 2 secs and re-download
                    if StatusCode::OK != res.status() {
                        if retry == 0 {
                            break;
                        }
                        retry -= 1;
                        tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                        continue;
                    }

                    let pb = mc.lock().await.add(ProgressBar::new(total_size));
                    pb.set_style(ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"));

                    pb.set_message(format!(
                        "[{}/{}] {} {}",
                        page.current,
                        page.total,
                        fname.purple(),
                        "downloading...".blue().bold()
                    ));

                    let mut stream = res.bytes_stream();
                    let mut downloaded: u64 = 0;

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(item) => {
                                file.write_all(&item)
                                    .await
                                    .context("Failed writes bytes to file")?;
                                let new = min(downloaded + (item.len() as u64), total_size);
                                downloaded = new;
                                pb.set_position(new);
                            }
                            Err(_) => continue,
                        }
                    }

                    file.flush().await.context("file.flush")?;

                    pb.finish_with_message(format!(
                        "[{}/{}] {} {}",
                        page.current,
                        page.total,
                        fname.purple(),
                        "success".green().bold()
                    ));

                    sleep(Duration::from_secs(1)).await;
                    pb.finish_and_clear();
                } else {
                    return Err(anyhow::anyhow!("Failed to send request for {}", path));
                }
                break;
            }
        }

        Ok(())
    }
}
