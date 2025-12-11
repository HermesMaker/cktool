use std::{thread, time::Duration};

use crate::{declare, link::Page, request};
use anyhow::Result;
use colored::Colorize;
use reqwest::StatusCode;

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

        let mut retry = self.retry;
        println!("Start fetching pages");

        // Fetch all post IDs from paginated API
        loop {
            print!("fetching {}", link.url().purple());

            let client = request::new()?;
            match client.get(link.url()).send().await {
                Ok(r) => {
                    if r.status() != StatusCode::OK {
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
                            if retry > 0 {
                                println!(" -- {} {}", "RETRY PARSE JSON".yellow(), confirm);
                                retry -= 1;
                                thread::sleep(Duration::from_secs(
                                    declare::TOO_MANY_REQUESTS_DELAY_SEC,
                                ));
                                continue;
                            }
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
