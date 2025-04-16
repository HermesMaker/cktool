use crate::{fetch::fetch_page, link::Link};
use anyhow::Result;
use futures_util::lock::Mutex;
use indicatif::MultiProgress;
use std::sync::Arc;

use super::{download_per_page::download_per_page, page_status::PageStatus};

/// Main function to download all content from a given URL
///
/// # Arguments
/// * `url` - The base URL to download from
/// * `split_dir` - Whether to split downloads into separate directories
/// * `task_limit` - Maximum number of concurrent download tasks
/// * `outdir` - Output directory for downloaded files
pub async fn all(link: Link, task_limit: usize, outdir: &str) -> Result<()> {
    let m = Arc::new(Mutex::from(MultiProgress::new()));

    let outdir = outdir.to_string();
    let mut posts_id = fetch_page(&link, &outdir).await?;
    // Process downloads in batches
    let mut page = PageStatus {
        current: 0,
        total: posts_id.len() as u32,
    };

    while !posts_id.is_empty() {
        let mut multi_task = tokio::task::JoinSet::new();

        while let Some(pid) = posts_id.pop() {
            let outdir = outdir.clone();
            let link = link.clone();
            let m = m.clone();

            while multi_task.len() >= task_limit {
                multi_task.join_next().await.unwrap().unwrap();
            }

            page.current += 1;
            let page = page.clone();

            multi_task.spawn(async move {
                if let Err(e) = download_per_page(&link.post_id(&pid), &outdir, m, page).await {
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
