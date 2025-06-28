use crate::{
    declare::{RetryType, TaskType},
    link::Link,
};
use anyhow::{Context, Result};
use futures_util::lock::Mutex;
use indicatif::MultiProgress;
use std::sync::Arc;
use tokio::fs;

use super::{info::DownloaderInfo, page_status::StatusBar};

#[derive(Clone)]
pub struct Downloader {
    pub link: Link,
    pub task_limit: TaskType,
    pub outdir: String,
    pub retry: RetryType,
    pub multi_progress: Arc<Mutex<MultiProgress>>,
    pub info: Arc<Mutex<DownloaderInfo>>,
}

impl Downloader {
    pub fn new(link: Link, task_limit: TaskType, outdir: String, retry: RetryType) -> Self {
        Self {
            link,
            task_limit,
            outdir,
            retry,
            multi_progress: Arc::new(Mutex::from(MultiProgress::new())),
            info: Arc::new(Mutex::new(DownloaderInfo::new())),
        }
    }

    /// Collect all posts id from sigle post or pages.
    pub async fn fetch_post_id(&self) -> Result<Vec<String>> {
        let posts_id = match self.link.typ {
            crate::link::UrlType::Post => {
                // Single post.
                vec![self.link.get_post_id().expect("invalid url").to_string()]
            }
            crate::link::UrlType::Page | crate::link::UrlType::None => self.fetch_page().await?,
        };
        Ok(posts_id)
    }

    /// Main function to download all content.
    pub async fn all(&mut self) -> Result<()> {
        let posts_id = self.fetch_post_id().await.context("Failed fetch post id")?;
        let posts_id = Arc::new(Mutex::new(posts_id));
        let posts_id_total = { posts_id.lock().await.len() };
        fs::create_dir_all(&self.outdir).await?;

        let mut multi_tasks = Vec::new();

        for _ in 0..self.task_limit {
            let mut self_instance = self.clone();
            let info = self.info.clone();
            let posts_id = posts_id.clone();
            multi_tasks.push(tokio::spawn(async move {
                loop {
                    let (pid, status) = {
                        let mut posts_id = posts_id.lock().await;
                        (
                            posts_id.pop(),
                            StatusBar {
                                queues: posts_id.len() as u32,
                                total: posts_id_total as u32,
                            },
                        )
                    };
                    if let Some(pid) = pid {
                        let result = self_instance.download_post(pid, status).await;
                        {
                            info.lock().await.integrate(&result);
                        }
                    } else {
                        break;
                    }
                }
            }));
        }

        for handle in multi_tasks {
            handle.await.unwrap()
        }

        Ok(())
    }
}
