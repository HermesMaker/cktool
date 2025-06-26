// private
mod download_per_page;
mod fetch_pages;
mod get_posts_from_page;
mod index;
mod page_status;

// public
pub use index::Downloader;
pub use page_status::PageStatus;
