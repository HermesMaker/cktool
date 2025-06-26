use crate::link::Page;
use anyhow::Result;
use colored::Colorize;

use super::Downloader;

impl Downloader {
    /// This function will collect all post id from page(s).
    pub async fn fetch_page(&self) -> Result<Vec<String>> {
        let mut posts_id = Vec::new();
        let mut confirm = 0;
        let mut link = self.link.clone();
        if let Page::All = link.page {
            link.set_page(0);
        }

        println!("Start fetching pages");

        // Fetch all post IDs from paginated API
        loop {
            print!("fetching {}", link.url().purple());

            match reqwest::get(&link.url()).await {
                Ok(r) => {
                    let len = r.content_length().unwrap_or(0);
                    if len < 10 {
                        confirm += 1;
                        if confirm < 3 {
                            println!(" -- {} {}", "CONFIRM".yellow(), confirm);
                            continue;
                        }
                        println!(" -- {}", "NONE".yellow().bold());
                        break;
                    }

                    confirm = 0;

                    if let Ok(content) = r.text().await {
                        if let Ok(obj) = json::parse(&content) {
                            posts_id.extend((0..obj.len()).map(|i| obj[i]["id"].to_string()));
                        } else {
                            println!("Cannot parse JSON: {}", content);
                        }
                    }
                    println!(" -- {}", "PASS".green().bold());
                }
                Err(_) => {
                    println!(" -- {}", "FAILED".red().bold());
                    return Err(anyhow::anyhow!("Failed to fetch page"));
                }
            }
            if let Page::One(_) = self.link.page {
                break;
            }
            link.page_increst();
        }
        Ok(posts_id)
    }
}
