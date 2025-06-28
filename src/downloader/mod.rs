// private
mod download_post;
mod fetch_pages;
mod get_posts_from_page;
mod index;
mod info;
mod page_status;
mod print;

// public
pub use index::Downloader;
pub use page_status::PageStatus;
