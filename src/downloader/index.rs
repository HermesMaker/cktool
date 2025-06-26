use crate::{
    declare::{RetryType, TaskType},
    link::Link,
};
use anyhow::{Context, Ok, Result};
use futures_util::lock::Mutex;
use indicatif::MultiProgress;
use std::sync::Arc;
use tokio::fs;

use super::page_status::PageStatus;

#[derive(Clone)]
pub struct Downloader {
    pub link: Link,
    pub task_limit: TaskType,
    pub outdir: String,
    pub retry: RetryType,
    pub multi_progress: Arc<Mutex<MultiProgress>>,
}

impl Downloader {
    pub fn new(link: Link, task_limit: TaskType, outdir: String, retry: RetryType) -> Self {
        Self {
            link,
            task_limit,
            outdir,
            retry,
            multi_progress: Arc::new(Mutex::from(MultiProgress::new())),
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
    pub async fn all(&self) -> Result<()> {
        let mut posts_id = self.fetch_post_id().await.context("Failed fetch post id")?;
        fs::create_dir_all(&self.outdir).await?;

        // Process downloads in batches
        let mut page = PageStatus {
            current: 0,
            total: posts_id.len() as u32,
        };

        while !posts_id.is_empty() {
            let mut multi_task = tokio::task::JoinSet::new();

            while let Some(pid) = posts_id.pop() {
                while multi_task.len() >= self.task_limit {
                    multi_task.join_next().await.unwrap().unwrap();
                }

                page.current += 1;
                let page = page.clone();
                // create downloader instance to assign to new thread.
                let downloader_instance = self.clone();

                multi_task.spawn(async move {
                    if let Err(e) = downloader_instance.download_per_page(pid, page).await {
                        eprintln!("Error downloading page: {}", e);
                    }
                });
            }

            while let Some(result) = multi_task.join_next().await {
                if let Err(err) = result {
                    eprintln!("Task error: {}", err);
                }
            }
        }

        Ok(())
    }
}
