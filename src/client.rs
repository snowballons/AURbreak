//! AUR client: fetches package feeds via RSS and RPC v5 API.
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

// RPC v5 Types

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageBasic {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "PackageBaseID")]
    pub package_base_id: i64,
    #[serde(rename = "PackageBase")]
    pub package_base: String,
    #[serde(rename = "Maintainer")]
    pub maintainer: Option<String>,
    #[serde(rename = "NumVotes")]
    pub num_votes: i32,
    #[serde(rename = "Popularity")]
    pub popularity: f64,
    #[serde(rename = "FirstSubmitted")]
    pub first_submitted: i64,
    #[serde(rename = "LastModified")]
    pub last_modified: i64,
    #[serde(rename = "OutOfDate")]
    pub out_of_date: Option<String>,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "URLPath")]
    pub url_path: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PackageDetailed {
    #[serde(flatten)]
    pub basic: PackageBasic,
    #[serde(rename = "Submitter")]
    pub submitter: String,
    #[serde(rename = "License")]
    pub license: Vec<String>,
    #[serde(rename = "Depends")]
    pub depends: Vec<String>,
    #[serde(rename = "MakeDepends")]
    pub make_depends: Vec<String>,
    #[serde(rename = "OptDepends")]
    pub opt_depends: Vec<String>,
    #[serde(rename = "CheckDepends")]
    pub check_depends: Vec<String>,
    #[serde(rename = "Provides")]
    pub provides: Vec<String>,
    #[serde(rename = "Conflicts")]
    pub conflicts: Vec<String>,
    #[serde(rename = "Replaces")]
    pub replaces: Vec<String>,
    #[serde(rename = "Groups")]
    pub groups: Vec<String>,
    #[serde(rename = "Keywords")]
    pub keywords: Vec<String>,
    #[serde(rename = "CoMaintainers")]
    pub co_maintainers: Vec<String>,
}

/// UI-friendly package detail (subset of PackageDetailed for display)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetailPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub maintainer: String,
    pub votes: i32,
    pub popularity: f64,
    pub url: String,
    pub license: Vec<String>,
    pub depends: Vec<String>,
    pub make_depends: Vec<String>,
    pub opt_depends: Vec<String>,
    pub check_depends: Vec<String>,
    pub provides: Vec<String>,
    pub conflicts: Vec<String>,
    pub replaces: Vec<String>,
    pub groups: Vec<String>,
    pub keywords: Vec<String>,
    pub co_maintainers: Vec<String>,
    pub submitter: String,
    pub first_submitted: i64,
    pub last_modified: i64,
    pub out_of_date: Option<String>,
}

impl DetailPackage {
    pub fn from_detailed(d: PackageDetailed) -> Self {
        Self {
            name: d.basic.name,
            version: d.basic.version,
            description: d.basic.description.unwrap_or_default(),
            maintainer: d.basic.maintainer.unwrap_or_default(),
            votes: d.basic.num_votes,
            popularity: d.basic.popularity,
            url: d.basic.url.unwrap_or_default(),
            license: d.license,
            depends: d.depends,
            make_depends: d.make_depends,
            opt_depends: d.opt_depends,
            check_depends: d.check_depends,
            provides: d.provides,
            conflicts: d.conflicts,
            replaces: d.replaces,
            groups: d.groups,
            keywords: d.keywords,
            co_maintainers: d.co_maintainers,
            submitter: d.submitter,
            first_submitted: d.basic.first_submitted,
            last_modified: d.basic.last_modified,
            out_of_date: d.basic.out_of_date,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub resultcount: i32,
    #[serde(rename = "type")]
    pub result_type: String,
    pub version: i32,
    pub results: Vec<PackageBasic>,
    pub error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InfoResult {
    pub resultcount: i32,
    #[serde(rename = "type")]
    pub result_type: String,
    pub version: i32,
    pub results: Vec<PackageDetailed>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchBy {
    Name,
    #[serde(rename = "name-desc")]
    NameDesc,
    Depends,
    CheckDepends,
    OptDepends,
    MakeDepends,
    Maintainer,
    Submitter,
}

impl Default for SearchBy {
    fn default() -> Self {
        SearchBy::NameDesc
    }
}

impl SearchBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            SearchBy::Name => "name",
            SearchBy::NameDesc => "name-desc",
            SearchBy::Depends => "depends",
            SearchBy::CheckDepends => "checkdepends",
            SearchBy::OptDepends => "optdepends",
            SearchBy::MakeDepends => "makedepends",
            SearchBy::Maintainer => "maintainer",
            SearchBy::Submitter => "submitter",
        }
    }

    pub fn all() -> [SearchBy; 8] {
        [
            SearchBy::Name,
            SearchBy::NameDesc,
            SearchBy::Depends,
            SearchBy::CheckDepends,
            SearchBy::OptDepends,
            SearchBy::MakeDepends,
            SearchBy::Maintainer,
            SearchBy::Submitter,
        ]
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            SearchBy::Name => "Name",
            SearchBy::NameDesc => "Name (desc)",
            SearchBy::Depends => "Depends",
            SearchBy::CheckDepends => "Check Depends",
            SearchBy::OptDepends => "Opt Depends",
            SearchBy::MakeDepends => "Make Depends",
            SearchBy::Maintainer => "Maintainer",
            SearchBy::Submitter => "Submitter",
        }
    }
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

/// Build a reqwest client with standard config.
fn aur_client() -> Result<reqwest::blocking::Client, FetchError> {
    reqwest::blocking::Client::builder()
        .user_agent("browse-arch/0.1.0 (aur browser)")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))
}

/// Search packages via RPC v5.
/// `arg` is the search query. `by` defaults to NameDesc if None.
pub fn search_packages(arg: &str, by: Option<SearchBy>) -> Result<SearchResult, FetchError> {
    let client = aur_client()?;
    let by_str = by.map(|b| b.as_str()).unwrap_or(SearchBy::NameDesc.as_str());

    let resp = client
        .get("https://aur.archlinux.org/rpc/v5/search")
        .query(&[("arg", arg), ("by", by_str)])
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let json = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| FetchError::Network(e.to_string()))
}

#[allow(dead_code)]
/// Get detailed info for a single package via RPC v5.
pub fn get_package_info(name: &str) -> Result<PackageDetailed, FetchError> {
    let client = aur_client()?;

    let resp = client
        .get(&format!("https://aur.archlinux.org/rpc/v5/info/{}", name))
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let json = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    let result: InfoResult = serde_json::from_str(&json).map_err(|e| FetchError::Network(e.to_string()))?;

    result
        .results
        .into_iter()
        .next()
        .ok_or_else(|| FetchError::Network("Package not found".to_string()))
}

#[allow(dead_code)]
/// Get detailed info for multiple packages via RPC v5.
/// Uses GET with repeated `arg[]` query parameters.
pub fn get_multi_info(names: &[&str]) -> Result<Vec<PackageDetailed>, FetchError> {
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let client = aur_client()?;
    let mut query = Vec::with_capacity(names.len());
    for name in names {
        query.push(("arg[]", *name));
    }

    let resp = client
        .get("https://aur.archlinux.org/rpc/v5/info")
        .query(&query)
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let json = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    let result: InfoResult = serde_json::from_str(&json).map_err(|e| FetchError::Network(e.to_string()))?;

    Ok(result.results)
}

/// Get package name suggestions (autocomplete) via RPC v5.
pub fn suggest_packages(prefix: &str) -> Result<Vec<String>, FetchError> {
    let client = aur_client()?;

    let resp = client
        .get(&format!("https://aur.archlinux.org/rpc/v5/suggest/{}", prefix))
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let json = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| FetchError::Network(e.to_string()))
}

#[allow(dead_code)]
/// Get package base name suggestions via RPC v5.
pub fn suggest_pkgbases(prefix: &str) -> Result<Vec<String>, FetchError> {
    let client = aur_client()?;

    let resp = client
        .get(&format!("https://aur.archlinux.org/rpc/v5/suggest-pkgbase/{}", prefix))
        .send()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let json = resp.text().map_err(|e| FetchError::Network(e.to_string()))?;
    serde_json::from_str(&json).map_err(|e| FetchError::Network(e.to_string()))
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

    #[test]
    fn test_search_by_serialization() {
        assert_eq!(SearchBy::Name.as_str(), "name");
        assert_eq!(SearchBy::NameDesc.as_str(), "name-desc");
        assert_eq!(SearchBy::Depends.as_str(), "depends");
        assert_eq!(SearchBy::Maintainer.as_str(), "maintainer");
        assert_eq!(SearchBy::Submitter.as_str(), "submitter");

        let all = SearchBy::all();
        assert_eq!(all.len(), 8);

        // Test label
        assert_eq!(SearchBy::Name.label(), "Name");
        assert_eq!(SearchBy::NameDesc.label(), "Name (desc)");
    }

    #[test]
    fn test_search_by_default() {
        let default: SearchBy = Default::default();
        assert_eq!(default, SearchBy::NameDesc);
    }

    #[test]
    fn test_package_basic_deserialization() {
        let json = r#"
        {
            "ID": 12345,
            "Name": "test-package",
            "Description": "A test package",
            "PackageBaseID": 100,
            "PackageBase": "test-package",
            "Maintainer": "tester",
            "NumVotes": 42,
            "Popularity": 1.5,
            "FirstSubmitted": 1609459200,
            "LastModified": 1704067200,
            "OutOfDate": null,
            "Version": "1.0-1",
            "URLPath": "/cgit/aur.git/snapshot/test-package.tar.gz",
            "URL": "https://example.com"
        }
        "#;
        let pkg: PackageBasic = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(pkg.id, 12345);
        assert_eq!(pkg.name, "test-package");
        assert_eq!(pkg.description, Some("A test package".to_string()));
        assert_eq!(pkg.package_base_id, 100);
        assert_eq!(pkg.maintainer, Some("tester".to_string()));
        assert_eq!(pkg.num_votes, 42);
        assert_eq!(pkg.popularity, 1.5);
        assert_eq!(pkg.first_submitted, 1609459200);
        assert_eq!(pkg.last_modified, 1704067200);
        assert_eq!(pkg.out_of_date, None);
        assert_eq!(pkg.version, "1.0-1");
    }

    #[test]
    fn test_package_basic_deserialization_minimal() {
        let json = r#"
        {
            "ID": 1,
            "Name": "minimal",
            "Description": null,
            "PackageBaseID": 1,
            "PackageBase": "minimal",
            "Maintainer": null,
            "NumVotes": 0,
            "Popularity": 0.0,
            "FirstSubmitted": 0,
            "LastModified": 0,
            "OutOfDate": null,
            "Version": "1.0-1",
            "URLPath": null,
            "URL": null
        }
        "#;
        let pkg: PackageBasic = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(pkg.name, "minimal");
        assert_eq!(pkg.description, None);
        assert_eq!(pkg.maintainer, None);
        assert_eq!(pkg.out_of_date, None);
        assert_eq!(pkg.url_path, None);
        assert_eq!(pkg.url, None);
    }

    #[test]
    fn test_search_result_deserialization() {
        let json = r#"
        {
            "resultcount": 2,
            "type": "search",
            "version": 5,
            "results": [
                {
                    "ID": 1,
                    "Name": "pkg1",
                    "Description": "First package",
                    "PackageBaseID": 1,
                    "PackageBase": "pkg1",
                    "Maintainer": "maint1",
                    "NumVotes": 10,
                    "Popularity": 0.5,
                    "FirstSubmitted": 1000,
                    "LastModified": 2000,
                    "OutOfDate": null,
                    "Version": "1.0-1",
                    "URLPath": "/path1",
                    "URL": "https://example.com/1"
                },
                {
                    "ID": 2,
                    "Name": "pkg2",
                    "Description": "Second package",
                    "PackageBaseID": 2,
                    "PackageBase": "pkg2",
                    "Maintainer": "maint2",
                    "NumVotes": 20,
                    "Popularity": 1.0,
                    "FirstSubmitted": 3000,
                    "LastModified": 4000,
                    "OutOfDate": "5000",
                    "Version": "2.0-1",
                    "URLPath": "/path2",
                    "URL": "https://example.com/2"
                }
            ],
            "error": null
        }
        "#;
        let result: SearchResult = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(result.resultcount, 2);
        assert_eq!(result.result_type, "search");
        assert_eq!(result.version, 5);
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].name, "pkg1");
        assert_eq!(result.results[1].name, "pkg2");
        assert_eq!(result.results[1].out_of_date, Some("5000".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_info_result_deserialization() {
        let json = r#"
        {
            "resultcount": 1,
            "type": "multiinfo",
            "version": 5,
            "results": [
                {
                    "ID": 1,
                    "Name": "detailed-pkg",
                    "Description": "A detailed package",
                    "PackageBaseID": 1,
                    "PackageBase": "detailed-pkg",
                    "Maintainer": "maint",
                    "NumVotes": 100,
                    "Popularity": 5.0,
                    "FirstSubmitted": 1000,
                    "LastModified": 2000,
                    "OutOfDate": null,
                    "Version": "1.0-1",
                    "URLPath": "/path",
                    "URL": "https://example.com",
                    "Submitter": "submitter",
                    "License": ["GPL-3.0"],
                    "Depends": ["dep1", "dep2"],
                    "MakeDepends": ["make1"],
                    "OptDepends": ["opt1: for something"],
                    "CheckDepends": ["check1"],
                    "Provides": ["provides1"],
                    "Conflicts": ["conflicts1"],
                    "Replaces": ["replaces1"],
                    "Groups": ["group1"],
                    "Keywords": ["keyword1"],
                    "CoMaintainers": ["comaint1"]
                }
            ],
            "error": null
        }
        "#;
        let result: InfoResult = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(result.resultcount, 1);
        assert_eq!(result.result_type, "multiinfo");
        assert_eq!(result.results.len(), 1);
        let pkg = &result.results[0];
        assert_eq!(pkg.basic.name, "detailed-pkg");
        assert_eq!(pkg.submitter, "submitter");
        assert_eq!(pkg.license, vec!["GPL-3.0"]);
        assert_eq!(pkg.depends, vec!["dep1", "dep2"]);
        assert_eq!(pkg.make_depends, vec!["make1"]);
        assert_eq!(pkg.opt_depends, vec!["opt1: for something"]);
        assert_eq!(pkg.check_depends, vec!["check1"]);
        assert_eq!(pkg.provides, vec!["provides1"]);
    }

    #[test]
    fn test_suggest_response_deserialization() {
        let json = r#"["yay","yay-bin","yay-git","yay-auto-review"]"#;
        let suggestions: Vec<String> = serde_json::from_str(json).expect("deserialize failed");
        assert_eq!(suggestions.len(), 4);
        assert_eq!(suggestions[0], "yay");
        assert_eq!(suggestions[1], "yay-bin");
    }

    #[test]
    #[ignore]
    fn smoke_live_search() {
        let res = search_packages("yay", Some(SearchBy::NameDesc)).expect("live search failed");
        println!("count={}", res.resultcount);
        for p in res.results.iter().take(3) {
            println!("{} | {} | votes={} pop={:.2}", p.name, p.version, p.num_votes, p.popularity);
        }
        assert!(res.resultcount > 0);
        assert!(!res.results.is_empty());
    }

    #[test]
    #[ignore]
    fn smoke_live_suggest() {
        let res = suggest_packages("yay").expect("live suggest failed");
        println!("suggestions: {:?}", res);
        assert!(!res.is_empty());
        assert!(res.iter().any(|s| s.starts_with("yay")));
    }
}