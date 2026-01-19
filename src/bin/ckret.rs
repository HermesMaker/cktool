use anyhow::{Context, anyhow};
use cktool::{
    declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC},
    request,
};
use clap::Parser;
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{StatusCode, header::RANGE};
use std::{cmp::min, path::Path, time::Duration};
use tokio::{
    fs,
    io::{AsyncWriteExt, BufWriter},
    time::sleep,
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
    let retry = args.retry.unwrap_or(100); // use for base value.
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
        if urls[i].is_empty() {
            continue;
        }
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
        let path_to_file = format!("{}/{}", out, file_name);
        let mut retry_request = retry;
        'request: loop {
            // check file exist to get fiel size to continue download by set range header.
            // if not found download from zero
            let (sender, file_size) = if let Ok(result) = Path::new(&path_to_file).try_exists()
                && result
            {
                let file_size = tokio::fs::metadata(&path_to_file)
                    .await
                    .context("cannot get file size")?
                    .len();
                let create_sender = request::new()?
                    .get(url)
                    .header(RANGE, format!("bytes={}-", file_size));
                (create_sender, Some(file_size))
            } else {
                (request::new()?.get(url), None)
            };

            // create or open file follow by file_size.
            let file = if file_size.is_some() {
                tokio::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(&path_to_file)
                    .await?
            } else {
                tokio::fs::File::create(&path_to_file).await?
            };
            let mut file = BufWriter::new(file);

            if let Ok(response) = sender.send().await {
                let total_size = match response.content_length().context("Cannot get total size") {
                    Ok(v) => v,
                    Err(_) => {
                        eprintln!("Failed receive file size");
                        if retry_request > 0 {
                            retry_request -= 1;
                            continue;
                        }
                        return Err(anyhow!("Failed download"));
                    }
                };

                // this download was clompleted.
                if 190 == response.status() || 416 == response.status() {
                    println!("{} {}", "success".green().bold(), file_name.blue());
                    break 'request;
                }
                // prevent too many requests
                if StatusCode::TOO_MANY_REQUESTS == response.status() {
                    tokio::time::sleep(Duration::from_secs(TOO_MANY_REQUESTS_DELAY_SEC)).await;
                    continue;
                }
                // prevent bad gateway: wait 2 secs and re-download
                if StatusCode::OK != response.status()
                    && StatusCode::PARTIAL_CONTENT != response.status()
                {
                    if retry_request == 0 {
                        return Err(anyhow!("Failed download"));
                    }
                    retry_request -= 1;
                    tokio::time::sleep(Duration::from_secs(ERROR_REQUEST_DELAY_SEC)).await;
                    continue;
                }
                let mut downloaded: u64 = file_size.unwrap_or(0);
                let p = create_progress_bar(total_size + downloaded);
                p.set_message(format!(
                    "[{}] {} {}",
                    index,
                    "Downloading".blue().bold(),
                    file_name
                ));

                let mut stream = response.bytes_stream();

                #[allow(unused)]
                'receive_item: while let Some(item) = stream.next().await {
                    match item {
                        Ok(item) => {
                            let mut retry_write = 10;
                            while (file.write_all(&item).await).is_err() && retry_write > 0 {
                                retry_write -= 1;
                            }

                            let new = min(downloaded + (item.len() as u64), total_size);
                            downloaded = new;
                            p.set_position(new);
                        }
                        Err(_) => {
                            // failed while downloading.
                            p.set_message(format!(
                                "[{}] {} {}",
                                index,
                                "Reconnect".yellow().bold(),
                                file_name
                            ));
                            sleep(Duration::from_secs(1)).await;
                            continue 'request;
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
            } else if retry_request > 0 {
                retry_request -= 1;
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
