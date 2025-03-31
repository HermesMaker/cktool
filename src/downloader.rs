use colored::Colorize;
use futures_util::{StreamExt, lock::Mutex};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{cmp::min, sync::Arc};
use tokio::{
    fs::{self, File},
    io::{AsyncWriteExt, BufWriter},
    time::{Duration, sleep},
};
use anyhow::{Result, Context};
use json::JsonValue;

#[derive(Clone)]
struct Link {
    domain: String,
    url: String,
}

impl Link {
    /// Parses a URL string into a Link struct
    ///
    /// # Arguments
    /// * `url` - The URL string to parse
    ///
    /// # Returns
    /// * `Result<Self>` - Returns Ok(Link) if parsing is successful, Err otherwise
    pub fn parse(url: String) -> Result<Self> {
        let url: Vec<&str> = url.split("?").collect();
        let url = url
            .first()
            .context("Cannot parse URL")?
            .replace(".su", ".su/api/v1");
        let domain = url.split(".su").collect::<Vec<&str>>();
        let domain = domain.first().context("Invalid domain")?;
        
        Ok(Self {
            domain: format!("{}.su", domain),
            url,
        })
    }
}

/// Represents pagination information for downloads
#[derive(Clone)]
struct Page {
    current: u32,
    total: u32,
}

/// Main function to download all content from a given URL
///
/// # Arguments
/// * `url` - The base URL to download from
/// * `split_dir` - Whether to split downloads into separate directories
/// * `task_limit` - Maximum number of concurrent download tasks
/// * `outdir` - Output directory for downloaded files
pub async fn all(url: &str, split_dir: bool, task_limit: usize, outdir: &str) -> Result<()> {
    let m = Arc::new(Mutex::from(MultiProgress::new()));
    let link = Link::parse(url.to_owned())?;
    
    let outdir = if split_dir {
        format!("{}/0", outdir)
    } else {
        outdir.to_string()
    };
    
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

    // Process downloads in batches
    let mut page = Page {
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
                let url = format!("{}/post/{}", link.url, pid);
                let link = Link {
                    domain: link.domain,
                    url,
                };
                if let Err(e) = download_per_page(&link, &outdir, m, page).await {
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

/// Fetches all post attachments from a specific page URL
///
/// # Arguments
/// * `url` - The URL of the post page
///
/// # Returns
/// * `Result<Vec<String>>` - Vector of file paths to download
async fn get_posts_from_page(url: &str) -> Result<Vec<String>> {
    let res = reqwest::get(url).await?;
    let text = res.text().await?;
    let obj = json::parse(&text)?;
    
    let mut posts = Vec::new();
    
    // Add main file if present
    if let Some(file) = obj["post"]["file"]["path"].as_str() {
        posts.push(file.to_string());
    }
    
    // Add attachments
    if let JsonValue::Array(attachments) = &obj["post"]["attachments"] {
        posts.extend(attachments.iter().filter_map(|a| a["path"].as_str().map(String::from)));
    }
    
    Ok(posts)
}

/// Downloads all files from a specific page
///
/// # Arguments
/// * `link` - The Link struct containing domain and URL information
/// * `outdir` - Output directory for downloaded files
/// * `m` - Progress tracking mutex
/// * `page` - Current page information for progress display
async fn download_per_page(link: &Link, outdir: &str, m: Arc<Mutex<MultiProgress>>, page: Page) -> Result<()> {
    let posts = get_posts_from_page(&link.url).await?;
    
    for path in posts {
        let outdir = outdir.to_owned();
        let mc = m.clone();
        let link = format!("{}{}", link.domain, path);
        let fname = path.split("/").last().context("Invalid file path")?;
        
        let client = reqwest::Client::new();
        let file = File::create(format!("{}/{}", outdir, fname)).await?;
        let mut file = BufWriter::new(file);
        
        if let Ok(res) = client.get(&link).send().await {
            let total_size = res.content_length().context("Cannot get total size")?;
            
            let pb = mc.lock().await.add(ProgressBar::new(total_size));
            pb.set_style(ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
                .progress_chars("#>-"));
            
            pb.set_message(format!(
                "[{}/{}] {} {}",
                page.current,
                page.total,
                fname.purple(),
                "downloading...".blue().bold()
            ));

            let mut stream = res.bytes_stream();
            let mut downloaded: u64 = 0;
            
            while let Some(item) = stream.next().await {
                let item = item?;
                file.write_all(&item).await?;
                let new = min(downloaded + (item.len() as u64), total_size);
                downloaded = new;
                pb.set_position(new);
            }
            
            file.flush().await?;
            
            pb.finish_with_message(format!(
                "[{}/{}] {} {}",
                page.current,
                page.total,
                fname.purple(),
                "success".green().bold()
            ));
            
            sleep(Duration::from_secs(1)).await;
            pb.finish_and_clear();
        } else {
            return Err(anyhow::anyhow!("Failed to send request for {}", path));
        }
    }
    
    Ok(())
}
