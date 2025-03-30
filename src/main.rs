// Import required dependencies for CLI argument parsing and shell completion
use cktool::downloader;
use clap::{CommandFactory, Parser, command};
use clap_complete::{Shell, generate};
use std::io;

/// Command line arguments structure for the cktool
#[derive(Parser)]
#[command(name = "cktool", version, about)]
struct Args {
    /// Output directory for downloaded content
    #[arg(short, long)]
    out: Option<String>,
    /// Number of concurrent download tasks
    #[arg(short, long, default_value_t = 8)]
    task: usize,
    /// Whether to split downloads into separate directories
    #[arg(short, long, default_value_t = false)]
    split: bool,
    /// URL of the profile account to download content from
    #[arg(short, long)]
    url: Option<String>,
    /// Generate shell completion scripts for the specified shell
    #[arg(short, long)]
    completion: Option<Shell>,
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
    if let Some(url) = args.url {
        // Determine output directory - use URL's last segment if not specified
        let out_dir = match args.out {
            Some(path) => path,
            None => url.split("/").last().unwrap().to_string(),
        };
        // Start the download process with specified parameters
        downloader::all(&url, args.split, args.task, &out_dir).await;
        println!("Download success");
    } else {
        // Show help message if no URL is provided
        let _ = Args::command().print_help();
    }
}
