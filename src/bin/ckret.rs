use anyhow::{Context, anyhow};
use cktool::{
    declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC},
    request,
};
use clap::Parser;
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::StatusCode;
use std::{cmp::min, path::Path, time::Duration};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
};

#[derive(Parser)]
#[command(
    name = "ckret",
    version,
    about = "Download contents from urls in file. Work with file created using the command 'cktool --log'."
)]
struct Args {
    /// path to file which store list of urls.
    file: String,
    /// Output directory for downloaded content.
    #[arg(short, long, value_name = "Folder")]
    out: Option<String>,
    /// specify the maximum number of re-download when failed.
    #[arg(short, long, default_value=None)]
    retry: Option<u32>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut urls = read_file(&args.file).await?;
    let retry = args.retry.unwrap_or(100);
    let out = match args.out {
        Some(_as) => _as,
        None => ".".to_string(),
    };

    println!("{}", "Parameters".green().bold());
    println!("{} {}", "File".blue().bold(), args.file);
    println!("{} {}", "retry".blue().bold(), retry);
    println!("{} {}", "out".blue().bold(), out);
    println!();

    // create foder
    if !Path::new(&out).is_dir() {
        tokio::fs::create_dir_all(&out)
            .await
            .context("Failed create directory")?;
    }

    let url_len = urls.len();
    for i in 0..url_len {
        if let Some(first_char) = urls[i].chars().next()
            && first_char == '#'
        {
            println!("{} {}", "skip".yellow().bold(), urls[i].blue());
            continue;
        }
        if download(&urls[i], retry, &out, i as u64).await.is_ok() {
            urls[i] = format!("#{}", urls[i]);
            fs::write(&args.file, urls.join("\n")).await?;
        } else {
            println!("{} {}", "Failed".red(), urls[i].red());
        }
    }
    println!("{}", "Done!".green());

    Ok(())
}

/// Read file from file and convert to Vec<String> by split '\n'
pub async fn read_file(file: &str) -> anyhow::Result<Vec<String>> {
    let file = tokio::fs::read_to_string(file).await?;
    let file: Vec<String> = file.split("\n").map(|x| x.to_string()).collect();
    Ok(file)
}

/// this func use to download each url.
pub async fn download(url: &str, retry: u32, out: &str, index: u64) -> anyhow::Result<()> {
    if let Some(file_name) = url.split("/").last() {
        let client = request::new()?;

        let file = tokio::fs::File::create(format!("{}/{}", out, file_name)).await?;
        let mut file = BufWriter::new(file);
        let mut retry = retry;
        loop {
            if let Ok(response) = client.get(url).send().await {
                let total_size = match response.content_length().context("Cannot get total size") {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Failed receive file size");
                        if retry > 0 {
                            retry -= 1;
                            continue;
                        }
                        return Err(anyhow!("Failed download"));
                    }
                };

                // prevent too many requests
                if StatusCode::TOO_MANY_REQUESTS == response.status() {
                    tokio::time::sleep(Duration::from_secs(TOO_MANY_REQUESTS_DELAY_SEC)).await;
                    continue;
                }
                // prevent bad gateway: wait 2 secs and re-download
                if StatusCode::OK != response.status() {
                    if retry == 0 {
                        return Err(anyhow!("Failed download"));
                    }
                    retry -= 1;
                    tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                    continue;
                }
                let p = create_progress_bar(total_size);
                p.set_message(format!(
                    "[{}] {} {}",
                    index,
                    "Downloading".blue().bold(),
                    file_name
                ));

                let mut stream = response.bytes_stream();
                let mut downloaded: u64 = 0;

                while let Some(item) = stream.next().await {
                    match item {
                        Ok(item) => {
                            if (file.write_all(&item).await).is_err() {
                                eprintln!("Failed writes bytes to file");
                                return Err(anyhow!("Failed download"));
                            }

                            let new = min(downloaded + (item.len() as u64), total_size);
                            downloaded = new;
                            p.set_position(new);
                        }
                        Err(_) => {
                            continue;
                        }
                    }
                }

                let _ = file.flush().await.context("file.flush");
                p.finish_with_message(format!(
                    "[{}] {} {}",
                    index,
                    "success".green().bold(),
                    file_name
                ));
            } else if retry > 0 {
                retry -= 1;
                tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                continue;
            } else {
                return Err(anyhow!("Failed download"));
            }
            break;
        }
    }
    Ok(())
}

pub fn create_progress_bar(len: u64) -> ProgressBar {
    let p = ProgressBar::new(len);
    p.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
            .progress_chars("#>-"));
    p
}
