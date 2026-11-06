//! AUR client: fetches package feeds via RSS.
//! Uses reqwest blocking on background threads so the Slint UI never blocks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AurPackage {
    pub name: String,
    pub version: String,
    pub votes: i32,
    pub popularity: f64,
    pub description: String,
    pub maintainer: String,
    pub updated_str: String,
    pub updated_utc: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RssPackage {
    pub name: String,
    pub description: String,
    pub link: String,
    pub pub_date: String,
}

#[derive(Debug)]
pub enum FetchError {
    Network(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Network(s) => write!(f, "Network error: {s}"),
        }
    }
}

pub fn fetch_last_updated() -> Result<Vec<AurPackage>, FetchError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("browse-arch/0.1.0 (aur browser)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let resp = client
        .get("https://aur.archlinux.org/rss/modified")
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let xml = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    let rss_items = parse_rss(&xml)?;

    Ok(rss_items
        .into_iter()
        .map(|r| AurPackage {
            name: r.name,
            version: String::new(),
            votes: 0,
            popularity: 0.0,
            description: r.description,
            maintainer: String::new(),
            updated_utc: parse_rfc2822_date(&r.pub_date),
            updated_str: r.pub_date,
        })
        .collect())
}

fn parse_rfc2822_date(s: &str) -> Option<DateTime<Utc>> {
    chrono::DateTime::parse_from_rfc2822(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

pub fn fetch_rss_newest() -> Result<Vec<RssPackage>, FetchError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("browse-arch/0.1.0 (aur browser)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let resp = client
        .get("https://aur.archlinux.org/rss/")
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let xml = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    parse_rss(&xml)
}

pub fn parse_rss(xml: &str) -> Result<Vec<RssPackage>, FetchError> {
    let mut items = Vec::with_capacity(100);

    for block in xml.split("<item>").skip(1) {
        let block = block.split("</item>").next().unwrap_or("");
        let title = extract_tag(block, "title").unwrap_or_default();
        let link = extract_tag(block, "link").unwrap_or_default();
        let description = extract_tag(block, "description").unwrap_or_default();
        let pub_date = extract_tag(block, "pubDate").unwrap_or_default();

        if !title.is_empty() {
            items.push(RssPackage {
                name: title,
                description,
                link,
                pub_date,
            });
        }
    }

    Ok(items)
}

fn extract_tag(xml_block: &str, tag: &str) -> Option<String> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = xml_block.find(&start_tag)? + start_tag.len();
    let end = xml_block.find(&end_tag)?;
    if start <= end {
        Some(xml_block[start..end].trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_rss_item() {
        let sample_rss = r#"
            <rss><channel>
            <item>
                <title>aerial-lock-git</title>
                <link>http://aur.archlinux.org/packages/aerial-lock-git</link>
                <description>Wayland session locker</description>
                <pubDate>Tue, 03 Nov 2026 03:15:41 +0000</pubDate>
            </item>
            </channel></rss>
        "#;
        let items = parse_rss(sample_rss).expect("rss parse failed");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "aerial-lock-git");
        assert_eq!(items[0].link, "http://aur.archlinux.org/packages/aerial-lock-git");
        assert_eq!(items[0].description, "Wayland session locker");
    }

    #[test]
    fn test_parse_rfc2822_date() {
        let dt = parse_rfc2822_date("Fri, 06 Nov 2026 10:12:39 +0000");
        assert!(dt.is_some());
        let dt = dt.unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 11);
        assert_eq!(dt.day(), 6);
    }

    #[test]
    fn test_parse_rfc2822_date_invalid() {
        assert!(parse_rfc2822_date("not a date").is_none());
    }
}