use crate::{
    declare::{RetryType, TaskType},
    link::Link,
};
use anyhow::{Context, Result};
use colored::Colorize;
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
    pub video_only: bool,
    pub image_only: bool,
    pub multi_progress: Arc<Mutex<MultiProgress>>,
    pub info: Arc<Mutex<DownloaderInfo>>,
    pub verbose: bool,
    pub creator_name: Arc<Mutex<Option<String>>>,
}

impl Downloader {
    pub fn print_parameters(&self) {
        println!("{}", "Parameters".green().bold());
        println!("{} {}", "Link".blue().bold(), self.link.url());
        println!("{} {}", "Outdir".blue().bold(), self.outdir);
        println!("{} {}", "TaskLimit".blue().bold(), self.task_limit);
        println!("{} {}", "Retry".blue().bold(), self.retry);
        println!("{} {}", "VideoOnly".blue().bold(), self.video_only);
        println!("{} {}", "ImageOnly".blue().bold(), self.image_only);
        println!("{} {}", "Verbose".blue().bold(), self.verbose);
        println!();
    }
    pub fn new(
        link: Link,
        task_limit: TaskType,
        outdir: String,
        retry: RetryType,
        video_only: bool,
        image_only: bool,
        verbose: bool,
    ) -> Self {
        Self {
            link,
            task_limit,
            outdir,
            retry,
            video_only,
            image_only,
            multi_progress: Arc::new(Mutex::from(MultiProgress::new())),
            info: Arc::new(Mutex::new(DownloaderInfo::new())),
            verbose,
            creator_name: Arc::new(Mutex::new(None)),
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
    pub async fn all(&mut self) -> anyhow::Result<()> {
        self.print_parameters();

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
                        if let Ok(result) = self_instance.download_post(pid, status).await {
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
