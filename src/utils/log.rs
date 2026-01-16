use tokio::{fs::File, io::AsyncWriteExt};

pub struct Log;

impl Log {
    /// This func will save failed url into file.
    pub async fn save_failed(failed_url: &[String], path: &str) {
        if let Ok(mut file) = File::create(path).await {
            for content in failed_url {
                let _ = file.write_all(format!("{}\n", content).as_bytes()).await;
            }
        }
    }
}
