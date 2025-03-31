use anyhow::{Context, Result};
#[derive(Clone)]
pub struct Link {
    pub domain: String,
    pub url: String,
}

impl Link {
    /// Parses a URL string into a Link struct
    ///
    /// # Arguments
    /// * `url` - The URL string to parse
    ///
    /// # Returns
    /// * `Result<Self>` - Returns Ok(Link) if parsing is successful, Err otherwise
    pub fn parse(url: String) -> Result<Self> {
        let url = url.replace(".su", ".su/api/v1");
        let domain = url.split(".su").collect::<Vec<&str>>();
        let domain = domain.first().context("Invalid domain")?;

        Ok(Self {
            domain: format!("{}.su", domain),
            url,
        })
    }

    /// remove '?o=' from url
    pub fn clear_option(&self) -> String {
        let url = self.url.split("?").collect::<Vec<&str>>();
        url.first()
            .expect("Cannot clear option from url")
            .to_string()
    }
}
