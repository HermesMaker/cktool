use anyhow::Context;
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

use crate::{
    declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC},
    request,
};
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "svg", "heic"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "webm", "mkv", "avi", "mov", "flv", "wmv", "mpg", "mpeg"];

use super::{Downloader, info::DownloaderInfo, page_status::StatusBar};

impl Downloader {
    /// Downloads all files in a specific post
    ///
    /// # Arguments
    /// * `pid` - post id to download
    /// * `page` - Current page information for progress display
    pub async fn download_post(
        &mut self,
        pid: String,
        status: StatusBar,
    ) -> anyhow::Result<DownloaderInfo> {
        let url = self.link.post_id(&pid);
        let posts = match self.get_posts_from_page(&url).await {
            Ok(v) => v,
            Err(_) => {
                {
                    self.info
                        .lock()
                        .await
                        .add_failed_file(url.replace("api/v1/", ""));
                }

                return Ok(DownloaderInfo::new());
            }
        };

        let mut download_info = DownloaderInfo::new();

        for path in posts {
            let outdir = self.outdir.clone();
            let mc = self.multi_progress.clone();
            let fname = if let Ok(v) = path.split("/").last().context("Invalid file path") {
                v
            } else {
                eprintln!("Invalid file path");
                continue;
            };

            // Filtering logic
            let file_extension = Path::new(&fname)
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase());

            let mut skip_file = false;
            if let Some(ext) = &file_extension {
                if self.video_only && !VIDEO_EXTENSIONS.contains(&ext.as_str()) {
                    skip_file = true;
                } else if self.image_only && !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                    skip_file = true;
                }
            } else if self.video_only || self.image_only {
                // If there's a filter but no extension, we skip.
                skip_file = true;
            }

            if skip_file {
                download_info.add_skip_file(path.clone()); // Assuming add_skipped_file exists or similar
                continue;
            }

            let client = request::new()?;
            let file = if let Ok(v) = File::create(format!("{}/{}", outdir, fname)).await {
                v
            } else {
                continue;
            };
            let mut file = BufWriter::new(file);

            let mut retry = self.retry;
            let mut retry_request = self.retry;
            loop {
                if let Ok(res) = client.get(&path).send().await {
                    let total_size = match res.content_length().context("Cannot get total size") {
                        Ok(v) => v,
                        Err(_) => {
                            eprintln!("Failed receive file size");
                            download_info.add_failed_file(path);
                            break;
                        }
                    };
                    // prevent too many requests
                    if StatusCode::TOO_MANY_REQUESTS == res.status() {
                        tokio::time::sleep(Duration::from_secs(TOO_MANY_REQUESTS_DELAY_SEC)).await;
                        continue;
                    }
                    // prevent bad gateway: wait 2 secs and re-download
                    if StatusCode::OK != res.status() {
                        if retry == 0 {
                            download_info.add_failed_file(path);
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
                        status.total,
                        status.queues,
                        fname.purple(),
                        "downloading...".blue().bold()
                    ));

                    let mut stream = res.bytes_stream();
                    let mut downloaded: u64 = 0;

                    while let Some(item) = stream.next().await {
                        match item {
                            Ok(item) => {
                                if (file.write_all(&item).await).is_err() {
                                    eprintln!("Failed writes bytes to file");
                                    download_info.add_failed_file(path.to_string());
                                    break;
                                }

                                let new = min(downloaded + (item.len() as u64), total_size);
                                downloaded = new;
                                pb.set_position(new);
                            }
                            Err(_) => {
                                download_info.add_failed_file(path.clone());
                                continue;
                            }
                        }
                    }

                    download_info.add_file_size(total_size);
                    download_info.add_success_file(1);
                    let _ = file.flush().await.context("file.flush");

                    pb.finish_with_message(format!(
                        "[{}/{}] {} {}",
                        status.total,
                        status.queues,
                        fname.purple(),
                        "success".green().bold()
                    ));

                    sleep(Duration::from_secs(1)).await;
                    pb.finish_and_clear();
                } else {
                    if retry_request == 0 {
                        download_info.add_failed_file(path);
                        break;
                    }
                    retry_request -= 1;
                    tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                    continue;
                }
                break;
            }
        }

        Ok(download_info)
    }
}
