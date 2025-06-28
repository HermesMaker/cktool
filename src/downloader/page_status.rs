/// Represents pagination information for downloads
#[derive(Clone)]
pub struct PageStatus {
    pub current: u32,
    pub total: u32,
}

#[derive(Clone)]
pub struct StatusBar {
    pub queues: u32,
    pub total: u32,
}
