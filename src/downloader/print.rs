use super::Downloader;
use colored::Colorize;
use size::Size;

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
