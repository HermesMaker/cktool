use clap::{CommandFactory, Parser, command};
use clap_complete::{Shell, generate};
use cktool::downloader;
use std::io;

#[derive(Parser)]
#[command(name = "cktool", version, about)]
struct Args {
    /// out directory
    #[arg(short, long)]
    out: Option<String>,
    #[arg(short, long, default_value_t = 8)]
    task: usize,
    #[arg(short, long, default_value_t = false)]
    split: bool,
    /// link of profile account to download
    #[arg(short, long)]
    url: Option<String>,
    /// generate auto complete for any shell
    #[arg(short, long)]
    completion: Option<Shell>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if let Some(shell) = &args.completion {
        let mut args_cli = Args::command();
        generate(*shell, &mut args_cli, "coom", &mut io::stdout());
        return;
    }
    if let Some(url) = args.url {
        let out_dir = match args.out {
            Some(path) => path,
            None => url.split("/").last().unwrap().to_string(),
        };
        downloader::all(&url, args.split, args.task, &out_dir).await;
        println!("Download success");
    } else {
        let _ = Args::command().print_help();
    }
    // let url = "https://coomer.su/api/v1/fansly/user/288758110233833472";
    // let task_limit = 100;
    // let split_dir = false;
}
