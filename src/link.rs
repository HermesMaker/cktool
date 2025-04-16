use anyhow::{Context, Ok, Result};

/// specific page download.
#[derive(Clone)]
pub enum Page {
    /// Download all pages.
    All,
    /// Download single page.
    One(u8),
}

#[derive(Clone)]
pub struct Link {
    pub domain: String,
    url: String,
    pub page: Page,
}

impl Link {
    pub fn url(&self) -> String {
        match self.page {
            Page::All => self.url.clone(),
            Page::One(page_number) => format!("{}?o={}", self.url, page_number * 50),
        }
    }
    /// create instance of Link.
    pub fn new(domain: String, url: String, page: Page) -> Self {
        Self { domain, url, page }
    }
    /// Parses a URL string into a Link struct
    ///
    /// # Arguments
    /// * `url` - The URL string to parse
    ///
    /// # Returns
    /// * `Result<Self>` - Returns Ok(Link) if parsing is successful, Err otherwise
    pub fn parse(url: String) -> Result<Self> {
        // convert to api path and clear params in url.
        let url = url.replace(".su", ".su/api/v1");
        let url = url.split("?").collect::<Vec<&str>>();
        let url = url.first().context("Invalid domain")?.to_string();

        let domain = url.split(".su").collect::<Vec<&str>>();
        let domain = domain.first().context("Invalid domain")?;

        Ok(Self::new(format!("{}.su", domain), url, Page::All))
    }

    /// remove '?o=' from url
    pub fn clear_option(&self) -> String {
        let url = self.url.split("?").collect::<Vec<&str>>();
        url.first()
            .expect("Cannot clear option from url")
            .to_string()
    }
    pub fn page_increst(&mut self) {
        if let Page::One(page_number) = self.page {
            self.page = Page::One(page_number + 1);
        }
    }
    pub fn set_page(&mut self, page_number: u8) {
        self.page = Page::One(page_number);
    }
    pub fn post_id(&self, post_id: &String) -> String {
        format!("{}/post/{}", self.url, post_id)
    }
}
