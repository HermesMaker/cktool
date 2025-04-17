use crate::link::Link;
use anyhow::Result;
use colored::Colorize;

pub async fn fetch_page(link: &Link, outdir: &str) -> Result<Vec<String>> {
    match link.page {
        crate::link::Page::All => fetch_all_pages(link, outdir).await,
        crate::link::Page::One(_) => fetch_one_page(link, outdir).await,
    }
}

async fn fetch_one_page(link: &Link, outdir: &str) -> Result<Vec<String>> {
    let mut posts_id = Vec::new();
    let mut confirm = 0;
    println!("Start fetching pages");
    print!("fetching {}", link.url().purple());
    loop {
        match reqwest::get(link.url()).await {
            Ok(r) => {
                let len = r.content_length().unwrap_or(0);
                if len < 10 {
                    println!(" -- {}", "NONE".yellow().bold());
                    return Err(anyhow::anyhow!("Page is NONE"));
                }

                if let Ok(content) = r.text().await {
                    if let Ok(obj) = json::parse(&content) {
                        posts_id.extend((0..obj.len()).map(|i| obj[i]["id"].to_string()));
                    } else {
                        println!("Cannot parse JSON: {}", content);
                    }
                }
                println!(" -- {}", "PASS".green().bold());
                break;
            }
            Err(_) => {
                println!(" -- {}", "FAILED".red().bold());
                if confirm > 2 {
                    return Err(anyhow::anyhow!("Failed to fetch page"));
                } else {
                    confirm += 1;
                }
            }
        }
    }
    Ok(posts_id)
}

async fn fetch_all_pages(link: &Link, outdir: &str) -> Result<Vec<String>> {
    let mut posts_id = Vec::new();
    let mut confirm = 0;
    let mut link = link.clone();
    link.set_page(0);

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
        link.page_increst();
    }
    Ok(posts_id)
}
