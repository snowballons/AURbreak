//! Offline cache for the AUR "last updated" feed.
//!
//! Stored as a single JSON file under the XDG data dir
//! (`~/.local/share/browse-arch/cache.json`).
//!
//! Writes go through a temp file + rename so a crash mid-write can never
//! leave a half-written file behind. A corrupt file is moved aside to
//! `cache.json.bak` and the app falls back to an empty cache instead of
//! dying.

use crate::client::AurPackage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    fetched_at: DateTime<Utc>,
    packages: Vec<CachedAurPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedAurPackage {
    name: String,
    version: String,
    votes: i32,
    popularity: f64,
    description: String,
    maintainer: String,
    updated_str: String,
    updated_utc: i64, // Unix timestamp as i64
}

#[derive(Debug)]
pub struct LoadedCache {
    pub fetched_at: DateTime<Utc>,
    pub packages: Vec<AurPackage>,
}

#[derive(Debug)]
pub enum LoadOutcome {
    /// No file yet (first run) or unreadable directory.
    Fresh,
    /// Parsed successfully.
    Loaded(LoadedCache),
    /// File was corrupt; moved aside, caller should treat as fresh.
    CorruptRecovered,
}

fn project_dir() -> PathBuf {
    directories::ProjectDirs::from("org", "aur", "browse-arch")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// Resolve the cache file path. Tests can override the project dir via the
/// `BROWSE_ARCH_TEST_DIR` env var so the real XDG data dir is never touched.
pub fn data_file() -> PathBuf {
    if let Ok(dir) = std::env::var("BROWSE_ARCH_TEST_DIR") {
        let p = PathBuf::from(dir);
        ensure_parent(&p);
        return p.join("cache.json");
    }
    project_dir().join("cache.json")
}

fn ensure_parent(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

pub fn load() -> LoadOutcome {
    let path = data_file();
    if !path.exists() {
        return LoadOutcome::Fresh;
    }
    match std::fs::read(&path) {
        Ok(bytes) => match serde_json::from_slice::<CacheFile>(&bytes) {
            Ok(cf) => {
                let packages = cf
                    .packages
                    .into_iter()
                    .filter_map(|cached| {
                        DateTime::from_timestamp(cached.updated_utc, 0).map(|dt| AurPackage {
                            name: cached.name,
                            version: cached.version,
                            votes: cached.votes,
                            popularity: cached.popularity,
                            description: cached.description,
                            maintainer: cached.maintainer,
                            updated_str: cached.updated_str,
                            updated_utc: Some(dt),
                        })
                    })
                    .collect();
                LoadOutcome::Loaded(LoadedCache {
                    fetched_at: cf.fetched_at,
                    packages,
                })
            }
            Err(_) => {
                let bak = path.with_extension("json.bak");
                let _ = std::fs::rename(&path, &bak);
                LoadOutcome::CorruptRecovered
            }
        },
        Err(_) => LoadOutcome::Fresh,
    }
}

pub fn save(packages: &[AurPackage], fetched_at: DateTime<Utc>) {
    let path = data_file();
    ensure_parent(&path);
    let cached: Vec<CachedAurPackage> = packages
        .iter()
        .map(|pkg| CachedAurPackage {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            votes: pkg.votes,
            popularity: pkg.popularity,
            description: pkg.description.clone(),
            maintainer: pkg.maintainer.clone(),
            updated_str: pkg.updated_str.clone(),
            updated_utc: pkg.updated_utc.map(|dt| dt.timestamp()).unwrap_or(0),
        })
        .collect();
    let cf = CacheFile {
        fetched_at,
        packages: cached,
    };
    let bytes = match serde_json::to_vec(&cf) {
        Ok(b) => b,
        Err(_) => return,
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &bytes).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, &path);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("browse-arch-test-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_pkg(name: &str) -> AurPackage {
        AurPackage {
            name: name.to_string(),
            version: "1.0-1".into(),
            votes: 5,
            popularity: 1.2,
            description: "test".into(),
            maintainer: "tester".into(),
            updated_str: "2026-01-01 00:00 (UTC)".into(),
            updated_utc: Some(
                chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_local_timezone(Utc)
                    .unwrap(),
            ),
        }
    }

    fn with_test_dir<F, R>(dir: &PathBuf, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let prev = std::env::var("BROWSE_ARCH_TEST_DIR").ok();
        std::env::set_var("BROWSE_ARCH_TEST_DIR", dir);
        let r = f();
        if let Some(p) = prev {
            std::env::set_var("BROWSE_ARCH_TEST_DIR", p);
        } else {
            std::env::remove_var("BROWSE_ARCH_TEST_DIR");
        }
        r
    }

    #[test]
    fn test_save_then_load_roundtrip() {
        let dir = tmp_dir();
        let pkgs = vec![sample_pkg("foo"), sample_pkg("bar")];
        let fetched = chrono::NaiveDate::from_ymd_opt(2026, 6, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap();

        with_test_dir(&dir, || {
            save(&pkgs, fetched);
            match load() {
                LoadOutcome::Loaded(loaded) => {
                    assert_eq!(loaded.packages.len(), 2);
                    assert_eq!(loaded.packages[0].name, "foo");
                    assert_eq!(loaded.packages[1].name, "bar");
                    assert_eq!(loaded.fetched_at, fetched);
                }
                other => panic!("expected Loaded, got {other:?}"),
            }
        });
    }

    #[test]
    fn test_corrupt_file_is_recovered() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            let path = data_file();
            ensure_parent(&path);
            fs::write(&path, b"not valid json {{{").unwrap();
            match load() {
                LoadOutcome::CorruptRecovered => {}
                other => panic!("expected CorruptRecovered, got {other:?}"),
            }
            assert!(!path.exists());
        });
    }
}