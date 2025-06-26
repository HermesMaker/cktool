// Import required dependencies for CLI argument parsing and shell completion
use cktool::{
    declare::{RetryType, TASK, TaskType},
    downloader::Downloader,
    link::{Link, Page},
};
use clap::{CommandFactory, Parser, command};
use clap_complete::{Shell, generate};
use colored::Colorize;
use std::io;

/// Command line arguments structure for the cktool
#[derive(Parser)]
#[command(name = "cktool", version, about)]
struct Args {
    /// Output directory for downloaded content
    #[arg(short, long, value_name = "Folder")]
    out: Option<String>,
    /// Number of concurrent download tasks
    #[arg(short, long, default_value_t = TASK)]
    task: TaskType,
    /// URL of the profile account or post to download content from
    #[arg(value_name = "url")]
    url: String,
    /// Generate shell completion scripts for the specified shell
    #[arg(short, long, value_name = "Shell")]
    completion: Option<Shell>,
    /// specific page downloading.
    #[arg(short,long, default_value=None, value_name="Number")]
    page: Option<u64>,
    /// specify the maximum number of re-download when failed.
    #[arg(short, long, default_value=None)]
    retry: Option<RetryType>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Handle shell completion generation if requested
    if let Some(shell) = &args.completion {
        let mut args_cli = Args::command();
        generate(*shell, &mut args_cli, "cktool", &mut io::stdout());
        return;
    }

    // Process download request if URL is provided
    // Determine output directory - use URL's last segment if not specified
    let out_dir = match args.out {
        Some(path) => path,
        None => {
            let url = args.url.split("/").last().unwrap().to_string();
            url.split("?")
                .collect::<Vec<&str>>()
                .first()
                .expect("cannot parse url")
                .to_string()
        }
    };
    if let Ok(mut link) = Link::parse(args.url) {
        if let Some(page) = args.page {
            // first page is zero.
            link.page = Page::One(page - 1);
        } else {
            link.page = Page::All
        }
        let retry = match args.retry {
            Some(re) => re,
            None => args.task as RetryType,
        };
        // Start the download process with specified parameters
        let downloader = Downloader::new(link, args.task, out_dir.clone(), retry);
        if downloader.all().await.is_ok() {
            println!("Download success to {}", out_dir.blue());
        }
    } else {
        eprintln!("Url is invalid");
    }
}
