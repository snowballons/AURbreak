//! AUR client: fetches the latest-updated package feed and individual package info.
//! Uses reqwest blocking on background threads so the Slint UI never blocks.

use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
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
    Parse(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Network(s) => write!(f, "Network error: {s}"),
            FetchError::Parse(s) => write!(f, "Parse error: {s}"),
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
        .get("https://aur.archlinux.org/packages?by=last-update-desc")
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let html_text = resp
        .text()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    parse_html_listing(&html_text)
}

pub fn parse_html_listing(html_text: &str) -> Result<Vec<AurPackage>, FetchError> {
    let document = Html::parse_document(html_text);
    let tr_sel = Selector::parse("tr").map_err(|e| FetchError::Parse(e.to_string()))?;
    let td_sel = Selector::parse("td").map_err(|e| FetchError::Parse(e.to_string()))?;
    let a_sel = Selector::parse("a").map_err(|e| FetchError::Parse(e.to_string()))?;

    // Pre-allocate for 50 rows (mem-with-capacity) — the HTML page always
    // returns 50 rows, so zero reallocations happen during parsing.
    let mut pkgs = Vec::with_capacity(50);

    for tr in document.select(&tr_sel) {
        let tds: Vec<_> = tr.select(&td_sel).collect();

        if tds.len() < 7 {
            continue;
        }

        let name = match tds[0].select(&a_sel).next() {
            Some(a) => a.text().collect::<String>().trim().to_string(),
            None => continue,
        };
        if name.is_empty() {
            continue;
        }

        let version = tds[1].text().collect::<String>().trim().to_string();
        let votes: i32 = tds[2]
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .unwrap_or(0);
        let popularity: f64 = tds[3]
            .text()
            .collect::<String>()
            .trim()
            .parse()
            .unwrap_or(0.0);
        let description = tds[4].text().collect::<String>().trim().to_string();

        let maintainer = tds[5]
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "orphan".to_string());

        let updated_raw = tds[6].text().collect::<String>().trim().to_string();
        // err-no-unwrap-prod: keep Option rather than defaulting silently to Utc::now()
        let updated_utc = parse_aur_date(&updated_raw);

        pkgs.push(AurPackage {
            name,
            version,
            votes,
            popularity,
            description,
            maintainer,
            updated_str: updated_raw,
            updated_utc,
        });
    }

    Ok(pkgs)
}

fn parse_aur_date(s: &str) -> Option<DateTime<Utc>> {
    let clean = s.trim_end_matches("(UTC)").trim();
    chrono::NaiveDateTime::parse_from_str(clean, "%Y-%m-%d %H:%M")
        .ok()
        .map(|naive| DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

pub fn fetch_rss_newest() -> Result<Vec<RssPackage>, FetchError> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("browse-arch/0.1.0 (aur browser)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let resp = client
        .get("https://aur.archlinux.org/rss")
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

    #[test]
    fn test_parse_html_row() {
        let sample_html = r#"
            <table>
            <tr>
                <td><a href="/packages/yay">yay</a></td>
                <td>13.1.0-1</td>
                <td>2649</td>
                <td>13.49</td>
                <td class="wrap">Yet another yogurt. Pacman wrapper.</td>
                <td><a href="/account/jguer">jguer</a></td>
                <td>2026-10-28 14:15 (UTC)</td>
            </tr>
            </table>
        "#;
        let pkgs = parse_html_listing(sample_html).expect("parse failed");
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "yay");
        assert_eq!(pkgs[0].version, "13.1.0-1");
        assert_eq!(pkgs[0].votes, 2649);
        assert_eq!(pkgs[0].maintainer, "jguer");
        assert_eq!(pkgs[0].description, "Yet another yogurt. Pacman wrapper.");
        assert!(pkgs[0].updated_utc.is_some());
    }

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
}