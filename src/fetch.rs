use crate::link::Link;
use anyhow::Result;
use colored::Colorize;
use tokio::fs;

pub async fn fetch_one_page(link: &Link, outdir: &str) -> Result<Vec<String>> {
    let mut posts_id = Vec::new();
    println!("Start fetching pages");
    print!("fetching {}", link.url.purple());
    match reqwest::get(&link.url).await {
        Ok(r) => {
            let len = r.content_length().unwrap_or(0);
            if len < 10 {
                println!(" -- {}", "NONE".yellow().bold());
                return Err(anyhow::anyhow!("Page is NONE"));
            }

            fs::create_dir_all(&outdir).await?;

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
    Ok(posts_id)
}

pub async fn fetch_all_pages(link: &Link, outdir: &str) -> Result<Vec<String>> {
    let mut posts_id = Vec::new();
    let mut i = 0;
    let mut confirm = 0;

    println!("Start fetching pages");

    // Fetch all post IDs from paginated API
    loop {
        let link = format!("{}?o={}", link.url, i * 50);
        print!("fetching {}", link.purple());

        match reqwest::get(&link).await {
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
                fs::create_dir_all(&outdir).await?;

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
        i += 1;
    }
    Ok(posts_id)
}
