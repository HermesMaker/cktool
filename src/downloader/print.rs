use std::time::Duration;

use crate::declare::{ERROR_REQUEST_DELAY_SEC, TOO_MANY_REQUESTS_DELAY_SEC};

use super::Downloader;
use colored::Colorize;
use indicatif::ProgressBar;
use size::Size;
use tokio::time::sleep;

impl Downloader {
    pub async fn failed_file(&self) -> Vec<String> {
        let info = self.info.lock().await;
        let failed_file = info.get_failed_file();
        failed_file.clone()
    }
    pub async fn print_reports(&self) {
        let info = self.info.lock().await;

        let failed_file = info.get_failed_file();
        let failed_file_len = failed_file.len();
        if !failed_file.is_empty() {
            for file in failed_file {
                println!(" {}\t{}", "Failed".red(), file.replace("api/v1/", "").red());
            }
        }

        let skip_file = info.get_skip_file();
        let skip_file_len = skip_file.len();
        if !skip_file.is_empty() {
            for file in skip_file {
                println!(
                    " {}\t{}",
                    "Skip".yellow(),
                    file.replace("api/v1/", "").yellow()
                );
            }
        }

        println!("Download success  to {} folder.", self.outdir.blue());
        let file_size = Size::from_bytes(info.get_file_size());
        println!("{}: {}", "Total size".blue(), file_size);
        println!("{}: {}", "success files".green(), info.get_success_file());
        println!("{}: {}", "Skipped files".yellow(), skip_file_len);
        println!("{}: {}", "Failed files".red(), failed_file_len);
    }
}

pub trait ProgressDisplay {
    fn download(&self, total: u32, queues: u32, download_counter_print: &str, fname: &str);
    fn retry_with_wait(&self, total: u32, queues: u32, fname: &str, download_counter: u64);
    fn wait(&self, total: u32, queues: u32, fname: &str);
    async fn finish_with_clear(&self, total: u32, queues: u32, fname: &str);
    fn reconnect(&self, total: u32, queues: u32, download_counter_print: &str, fname: &str);
    async fn failed(&self, total: u32, queues: u32, fname: &str);
    async fn was_done(&self, total: u32, queues: u32, fname: &str);
}

impl ProgressDisplay for ProgressBar {
    async fn was_done(&self, total: u32, queues: u32, fname: &str) {
        self.finish_with_message(format!(
            "[{}/{}] {} {}",
            total,
            queues,
            fname.purple(),
            "was done...".green().bold()
        ));
        sleep(Duration::from_millis(500)).await;
        self.finish_and_clear();
    }
    async fn failed(&self, total: u32, queues: u32, fname: &str) {
        self.set_message(format!(
            "[{}/{}] {} {}",
            total,
            queues,
            fname.purple(),
            "Failed".red().bold(),
        ));
        sleep(Duration::from_secs(1)).await;
        self.finish_and_clear();
    }
    fn download(&self, total: u32, queues: u32, download_counter_print: &str, fname: &str) {
        self.set_message(format!(
            "[{}/{}] {}{} {}",
            total,
            queues,
            download_counter_print.yellow(),
            fname.purple(),
            "downloading...".blue().bold()
        ));
    }

    fn retry_with_wait(&self, total: u32, queues: u32, fname: &str, download_counter: u64) {
        self.set_message(format!(
            "[{}/{}] {} {}[{}] {} {} secs.",
            total,
            queues,
            fname.purple(),
            "retry".blue().bold(),
            download_counter,
            "wait".yellow().bold(),
            ERROR_REQUEST_DELAY_SEC
        ));
    }

    fn wait(&self, total: u32, queues: u32, fname: &str) {
        self.set_message(format!(
            "[{}/{}] {} {} {} secs.",
            total,
            queues,
            fname.purple(),
            "wait".yellow().bold(),
            TOO_MANY_REQUESTS_DELAY_SEC.to_string().yellow()
        ));
    }

    async fn finish_with_clear(&self, total: u32, queues: u32, fname: &str) {
        self.finish_with_message(format!(
            "[{}/{}] {} {}",
            total,
            queues,
            fname.purple(),
            "success".green().bold()
        ));

        sleep(Duration::from_secs(1)).await;
        self.finish_and_clear();
    }

    fn reconnect(&self, total: u32, queues: u32, download_counter_print: &str, fname: &str) {
        self.set_message(format!(
            "[{}/{}] {}{} {}",
            total,
            queues,
            download_counter_print.yellow(),
            fname.purple(),
            "Reconnect...".yellow().bold()
        ));
    }
}
