use anyhow::{Context, Ok, Result};

/// specific page download.
#[derive(Clone, Debug)]
pub enum Page {
    /// Download all pages.
    All,
    /// Download single page.
    One(u64),
}

#[derive(Clone, Debug)]
pub enum UrlType {
    /// pages means the page showing the list of posts.
    Page,
    /// post is a page that displays content.
    Post,
    /// `None` means do not need to uses this value.
    None,
}

impl UrlType {
    /// Create instance of UrlType by Url
    /// this method just detect 'post' word from url
    pub fn parse(url: &str) -> Self {
        if url.contains("post") {
            UrlType::Post
        } else {
            UrlType::Page
        }
    }
}

#[derive(Clone, Debug)]
pub struct Link {
    pub domain: String,
    url: String,
    pub page: Page,
    pub typ: UrlType,
}

impl Link {
    pub fn url(&self) -> String {
        if let UrlType::Post = self.typ {
            return self.url.clone();
        }
        match self.page {
            Page::All => self.url.clone(),
            Page::One(page_number) => format!("{}?o={}", self.url, page_number * 50),
        }
    }
    /// create instance of Link.
    pub fn new(domain: String, url: String, page: Page, typ: UrlType) -> Self {
        Self {
            domain,
            url,
            page,
            typ,
        }
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
        // remove parameters from url
        let url = url.split("?").collect::<Vec<&str>>();
        let url = url.first().context("Invalid domain")?.to_string();

        let domain = url.split(".su").collect::<Vec<&str>>();
        let domain = domain.first().context("Invalid domain")?;

        Ok(Self::new(
            format!("{}.su", domain),
            url.clone(),
            Page::All,
            UrlType::parse(&url),
        ))
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
    pub fn set_page(&mut self, page_number: u64) {
        self.page = Page::One(page_number);
    }
    /// produces Url with post id
    ///
    /// example returned value `https://example.com/user/postid/`
    pub fn post_id(&self, post_id: &String) -> String {
        if let UrlType::Post = self.typ {
            return self.url.clone();
        }
        format!("{}/post/{}", self.url, post_id)
    }

    pub fn get_post_id(&self) -> Option<&str> {
        if let UrlType::Post = self.typ {
            let mut url_split = self.url.split("/").collect::<Vec<&str>>();
            if let Some(post_id) = url_split.pop() {
                // check if last element is empty &str
                if post_id.is_empty() {
                    return url_split.pop();
                }
                return Some(post_id);
            }
        }
        None
    }
}
