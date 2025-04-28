pub type RetryType = u32;

pub type TaskType = usize;
pub const TASK: usize = 8;

// delay before re-download after found 'too many requests' error.
pub const TOO_MANY_REQUESTS_DELAY_SEC: u64 = 10;
// delay before re-download after found 'any' request error.
pub const ERROR_REQUEST_DELAY_SEC: u64 = 2;
