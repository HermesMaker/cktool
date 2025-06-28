#[derive(Clone)]
pub struct DownloaderInfo {
    file_size: u64,
    success_file: u64,
    /// Some of files could not be download. So just skip it and manual download.
    skip_file: Vec<String>,
    /// keeps failed downloaded url.
    failed_file: Vec<String>,
}

impl DownloaderInfo {
    pub fn new() -> Self {
        Self {
            file_size: 0,
            success_file: 0,
            skip_file: Vec::new(),
            failed_file: Vec::new(),
        }
    }

    pub fn integrate(&mut self, dinfo: &Self) {
        self.file_size += dinfo.file_size;
        self.success_file += dinfo.success_file;
        self.failed_file.append(&mut dinfo.failed_file.clone());
        self.skip_file.append(&mut dinfo.skip_file.clone());
    }

    pub fn get_file_size(&self) -> u64 {
        self.file_size
    }

    pub fn get_success_file(&self) -> u64 {
        self.success_file
    }

    pub fn get_skip_file(&self) -> Vec<String> {
        self.skip_file.clone()
    }

    pub fn get_failed_file(&self) -> Vec<String> {
        self.failed_file.clone()
    }

    pub fn add_file_size(&mut self, file_size: u64) {
        self.file_size += file_size;
    }

    pub fn add_success_file(&mut self, success_file: u64) {
        self.success_file += success_file;
    }

    pub fn add_skip_file(&mut self, url: String) {
        self.skip_file.push(url);
    }

    pub fn add_failed_file(&mut self, url: String) {
        self.failed_file.push(url);
    }
}

impl Default for DownloaderInfo {
    fn default() -> Self {
        Self::new()
    }
}
