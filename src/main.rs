// Import required dependencies for CLI argument parsing and shell completion
use cktool::{
    declare::{RetryType, TASK, TaskType},
    downloader::Downloader,
    link::{Link, Page},
    utils::Log,
};
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};
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
    url: Option<String>,
    /// Generate shell completion scripts for the specified shell
    #[arg(short, long, value_name = "Shell")]
    completion: Option<Shell>,
    /// specific page downloading.
    #[arg(short,long, default_value=None, value_name="Number")]
    page: Option<u64>,
    /// specify the maximum number of re-download when failed.
    #[arg(short, long, default_value=None)]
    retry: Option<RetryType>,
    /// Download only video files
    #[arg(short = 'v', long, default_value_t = false)]
    video_only: bool,
    /// Download only image files
    #[arg(short = 'i', long, default_value_t = false)]
    image_only: bool,
    /// enable verbose logging
    #[arg(long, default_value_t = false)]
    verbose: bool,
    /// save failed posts to file
    #[arg(long,short, value_name="File", default_value = None)]
    log: Option<Option<String>>,
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

    if let Some(url) = args.url {
        if args.video_only && args.image_only {
            eprintln!("Error: Cannot use --video-only and --image-only together.");
            return;
        }
        // Process download request if URL is provided
        // Determine output directory - use URL's last segment if not specified
        let out_dir = match args.out {
            Some(path) => path,
            None => {
                let url = url.split("/").last().unwrap().to_string();
                url.split("?")
                    .collect::<Vec<&str>>()
                    .first()
                    .expect("cannot parse url")
                    .to_string()
            }
        };
        if let Ok(mut link) = Link::parse(url) {
            if let Some(page) = args.page {
                // first page is zero.
                link.page = Page::One(page - 1);
            } else {
                link.page = Page::All
            }
            let retry = match args.retry {
                Some(re) => re,
                None => (args.task as RetryType) * 10,
            };
            // Start the download process with specified parameters
            let mut downloader = Downloader::new(
                link,
                args.task,
                out_dir.clone(),
                retry,
                args.video_only,
                args.image_only,
                args.verbose,
            );
            match downloader.all().await {
                Ok(_) => {
                    downloader.print_reports().await;
                    // if log flag is exist.
                    if let Some(log) = args.log {
                        let log_file = if let Some(log) = log {
                            log
                        } else {
                            // default log's file name.
                            "failed.log".to_string()
                        };
                        let failed_files = downloader.failed_file().await;
                        if !failed_files.is_empty() {
                            Log::save_failed(&failed_files, &log_file).await;
                        }
                    }
                }
                Err(err) => eprintln!("{}", err),
            }
        } else {
            eprintln!("Url is invalid");
        }
    } else {
        let _ = Args::command().print_help();
    }
}
