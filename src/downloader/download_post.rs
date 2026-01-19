use anyhow::Context;
use chrono::Local;
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{StatusCode, header::RANGE, retry};
use std::{cmp::min, path};
use tokio::{
    fs::OpenOptions,
    io::{AsyncWriteExt, BufWriter},
    time::{Duration, sleep},
};

use crate::{
    declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC},
    downloader::print::ProgressDisplay,
    request,
};
use std::path::Path;

const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "tiff", "svg", "heic",
];
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "webm", "mkv", "avi", "mov", "flv", "wmv", "mpg", "mpeg", "m4v",
];

use super::{Downloader, info::DownloaderInfo, page_status::StatusBar};

impl Downloader {
    async fn log_status(
        &self,
        post_url: &str,
        file_name: &str,
        status: &str,
    ) -> anyhow::Result<()> {
        if self.verbose {
            let date = Local::now().format("%Y-%m-%d");
            let creator = self.creator_name.lock().await.clone().unwrap_or_default();
            let log_file_name = format!("{}_{}.log", date, creator);
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_name)
                .await?;
            let log_entry = format!(
                "Post URL: {}, File: {}, Status: {}\n",
                post_url, file_name, status
            );
            file.write_all(log_entry.as_bytes()).await?;
        }
        Ok(())
    }
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

            let skip_file = if let Some(ext) = &file_extension
                && (self.video_only && !VIDEO_EXTENSIONS.contains(&ext.as_str())
                    || self.image_only && !IMAGE_EXTENSIONS.contains(&ext.as_str()))
                && (self.video_only || self.image_only)
            {
                true
            } else {
                false
            };

            if skip_file {
                download_info.add_skip_file(path.clone()); // Assuming add_skipped_file exists or similar
                self.log_status(&url, fname, "skipped").await?;
                continue;
            }

            let path_to_file = format!("{}/{}", outdir, fname);

            let mut retry = self.retry;
            let mut retry_request = self.retry;
            let mut download_counter = 0;
            'request: loop {
                download_counter += 1;
                let url = url.clone();
                let (sender, file_size) = if let Ok(result) = Path::new(&path_to_file).try_exists()
                    && result
                {
                    let file_size = tokio::fs::metadata(&path_to_file)
                        .await
                        .context("cannot get file size")?
                        .len();
                    let create_sender = request::new()?
                        .get(&path)
                        .header(RANGE, format!("bytes={}-", file_size));
                    (create_sender, Some(file_size))
                } else {
                    (request::new()?.get(&path), None)
                };

                // create or open file follow by file_size.
                let file = if file_size.is_some() {
                    tokio::fs::OpenOptions::new()
                        .write(true)
                        .append(true)
                        .open(&path_to_file)
                        .await?
                } else {
                    tokio::fs::File::create(&path_to_file).await?
                };
                let mut file = BufWriter::new(file);

                if let Ok(res) = sender.send().await {
                    let mut downloaded: u64 = file_size.unwrap_or(0);
                    let total_size = match res.content_length().context("Cannot get total size") {
                        Ok(v) => v,
                        Err(_) => {
                            println!("{:?}", res.headers());
                            eprintln!("Failed receive file size status: {}", res.status());
                            if retry_request > 0 {
                                retry_request -= 1;
                                continue;
                            }
                            download_info.add_failed_file(path.clone());
                            self.log_status(&url, fname, "failed").await?;
                            break;
                        }
                    };
                    let pb = mc
                        .lock()
                        .await
                        .add(ProgressBar::new(total_size + downloaded));
                    pb.set_style(ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"));

                    // this download was clompleted.
                    if 190 == res.status() || 416 == res.status() {
                        pb.was_done(status.total, status.queues, fname).await;
                        break 'request;
                    }

                    // prevent too many requests
                    if StatusCode::TOO_MANY_REQUESTS == res.status() {
                        pb.wait(status.total, status.queues, fname);
                        tokio::time::sleep(Duration::from_secs(TOO_MANY_REQUESTS_DELAY_SEC)).await;
                        continue;
                    }
                    // prevent bad gateway: wait 2 secs and re-download
                    if StatusCode::OK != res.status() && StatusCode::PARTIAL_CONTENT != res.status()
                    {
                        if retry == 0 {
                            pb.failed(status.total, status.queues, fname).await;
                            download_info.add_failed_file(path.clone());
                            self.log_status(&url, fname, "failed").await?;
                            break;
                        }
                        pb.retry_with_wait(status.total, status.queues, fname, download_counter);
                        retry -= 1;
                        tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                        continue;
                    }

                    let download_counter_print = if download_counter > 1 {
                        format!("[{}]", download_counter)
                    } else {
                        String::new()
                    };
                    pb.download(status.total, status.queues, &download_counter_print, fname);
                    let mut stream = res.bytes_stream();

                    #[allow(unused_labels)]
                    'receive_item: while let Some(item) = stream.next().await {
                        match item {
                            Ok(item) => {
                                if (file.write_all(&item).await).is_err() {
                                    eprintln!("Failed writes bytes to file");
                                    download_info.add_failed_file(path.to_string());
                                    self.log_status(&url, fname, "failed").await?;
                                    break;
                                }

                                let new = min(downloaded + (item.len() as u64), total_size);
                                downloaded = new;
                                pb.set_position(new);
                            }
                            Err(_) => {
                                // failed while downloading.
                                if retry_request > 0 {
                                    retry_request -= 1;
                                    pb.reconnect(
                                        status.total,
                                        status.queues,
                                        &download_counter_print,
                                        fname,
                                    );
                                    sleep(Duration::from_secs(1)).await;
                                    continue;
                                }
                                download_info.add_failed_file(path.clone());
                                pb.failed(status.total, status.queues, fname).await;
                                break 'request;
                            }
                        }
                    }

                    download_info.add_file_size(total_size);
                    download_info.add_success_file(1);
                    self.log_status(&url, fname, "success").await?;
                    let _ = file.flush().await.context("file.flush");

                    pb.finish_with_clear(status.total, status.queues, fname)
                        .await;
                } else {
                    if retry_request == 0 {
                        download_info.add_failed_file(path.clone());
                        self.log_status(&url, fname, "failed").await?;
                        break;
                    }
                    retry_request -= 1;
                    tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                    continue;
                }
                // break;
            }
        }

        Ok(download_info)
    }
}
