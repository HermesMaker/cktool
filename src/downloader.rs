use colored::Colorize;
use futures_util::{StreamExt, lock::Mutex};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{cmp::min, process::exit, sync::Arc};
use tokio::{
    fs::{self, File},
    io::AsyncWriteExt,
    time::{Duration, sleep},
};

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
    /// * `Option<Self>` - Returns Some(Link) if parsing is successful, None otherwise
    pub fn parse(url: String) -> Option<Self> {
        let url: Vec<&str> = url.split("?").collect();
        let url = url
            .first()
            .expect("Cannot parse Url")
            .replace(".su", ".su/api/v1");
        let domain = url.split(".su").collect::<Vec<&str>>();
        if let Some(domain) = domain.first() {
            Some(Self {
                domain: format!("{}.su", domain),
                url,
            })
        } else {
            None
        }
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
pub async fn all(url: &String, split_dir: bool, task_limit: usize, outdir: &String) {
    // Initialize progress tracking
    let m = Arc::new(Mutex::from(MultiProgress::new()));
    let link = match Link::parse(url.to_owned()) {
        Some(l) => l,
        None => {
            println!("Url is invalid");
            exit(1);
        }
    };
    let mut i = 0;
    println!("Start fetching pages");
    let outdir = if split_dir {
        format!("{}/{}", outdir, i)
    } else {
        outdir.clone()
    };
    let mut posts_id = Vec::new();

    // Fetch all post IDs from paginated API
    let mut confirm = 0;
    loop {
        let link = format!("{}?o={}", link.url, i * 50);
        print!("fetching {}", link.purple());
        if let Ok(r) = reqwest::get(link).await {
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
            let _ = fs::create_dir_all(&outdir).await;
            if let Ok(content) = r.text().await {
                if let Ok(obj) = json::parse(&content) {
                    let len = obj.len();
                    for i in 0..len {
                        posts_id.push(obj[i]["id"].to_string());
                    }
                } else {
                    println!("cannot parse json {}", content);
                }
            } else {
                println!("Cannot get text content");
            }
            println!(" -- {}", "PASS".green().bold());
        } else {
            println!(" -- {}", "FAILED".red().bold());
            exit(101);
        }

        i += 1;
    }

    // Process downloads in batches with concurrent tasks
    let mut page = Page {
        current: 0,
        total: posts_id.len() as u32,
    };
    while posts_id.is_empty() {
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
                download_per_page(&link, &outdir, m, page).await;
            });
        }
        while let Some(result) = multi_task.join_next().await {
            match result {
                Ok(_) => (),
                Err(err) => println!("ERROR: multi_task line 81 {}", err),
            }
        }
    }
}

/// Fetches all post attachments from a specific page URL
///
/// # Arguments
/// * `url` - The URL of the post page
///
/// # Returns
/// * `Vec<String>` - Vector of file paths to download
async fn get_posts_from_page(url: &String) -> Vec<String> {
    let res = reqwest::get(url).await;
    let mut posts: Vec<String> = Vec::new();
    if let Ok(res) = res {
        if let Ok(text) = res.text().await {
            if let Ok(obj) = json::parse(&text) {
                // Download main file
                let file = obj["post"]["file"]["path"].clone();
                if !file.is_null() {
                    let path = file.to_string();
                    posts.push(path);
                }
                // Download additional attachments
                let len = obj["post"]["attachments"].len();
                for i in 0..len {
                    let path = obj["post"]["attachments"][i]["path"].clone().to_string();
                    posts.push(path);
                }
            } else {
                println!("ERROR: Cannot parse json, {}", text);
            }
        } else {
            println!("Cannot get bytes from {}", url);
        }
    } else {
        println!("ERROR: Failed request from {}", url);
    }
    posts
}

/// Downloads all files from a specific page
///
/// # Arguments
/// * `link` - The Link struct containing domain and URL information
/// * `outdir` - Output directory for downloaded files
/// * `m` - Progress tracking mutex
/// * `page` - Current page information for progress display
async fn download_per_page(link: &Link, outdir: &str, m: Arc<Mutex<MultiProgress>>, page: Page) {
    let posts = get_posts_from_page(&link.url).await;
    for path in posts {
        let outdir = outdir.to_owned();
        let mc = m.clone();
        let link = format!("{}{}", link.domain, path);
        let fname = path
            .split("/")
            .last()
            .expect("Something wrong with url at <download>(fn)");
        let client = reqwest::Client::new();
        if let Ok(mut file) = File::create(format!("{}/{}", outdir, fname)).await {
            if let Ok(res) = client.get(link).send().await {
                let total_size = match res.content_length() {
                    Some(ts) => ts,
                    None => {
                        println!("Cannot get total size");
                        return;
                    }
                };
                // Set up progress bar for this download
                let pb = mc.lock().await.add(ProgressBar::new(total_size));
                pb.set_style(ProgressStyle::default_bar()
                        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
                        .progress_chars("#>-"));
                pb.set_message(format!(
                    "[{}/{}] {} {}",
                    page.current,
                    page.total,
                    fname.purple(),
                    "downloading...".blue().bold()
                ));

                // Stream and write the file with progress updates
                let mut stream = res.bytes_stream();
                let mut downloaded: u64 = 0;
                while let Some(item) = stream.next().await {
                    if let Ok(item) = item {
                        let _ = file.write_all(&item).await;
                        let new = min(downloaded + (item.len() as u64), total_size);
                        downloaded = new;
                        pb.set_position(new);
                    } else {
                        println!("Failed to download {}", path);
                    }
                }
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
                println!("Client failed send {}", path)
            }
        } else {
            println!("Failed Create file");
        }
    }
}
