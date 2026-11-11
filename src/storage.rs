//! Offline cache for the AUR "last updated" feed.
//!
//! Stored as a single JSON file under the XDG data dir
//! (`~/.local/share/aurbreak/cache.json`).
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
    if let Some(dirs) = directories::ProjectDirs::from("org", "aur", "aurbreak") {
        let new_dir = dirs.data_dir().to_path_buf();
        // Seamless migration: if new aurbreak dir does not exist yet, check legacy browse-arch dir
        if !new_dir.exists()
            && let Some(old_dirs) = directories::ProjectDirs::from("org", "aur", "browse-arch")
        {
            let old_dir = old_dirs.data_dir().to_path_buf();
            if old_dir.exists() {
                let _ = std::fs::create_dir_all(&new_dir);
                if let Ok(entries) = std::fs::read_dir(&old_dir) {
                    for entry in entries.flatten() {
                        let _ = std::fs::copy(entry.path(), new_dir.join(entry.file_name()));
                    }
                }
            }
        }
        new_dir
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Resolve the cache file path. Tests can override the project dir via the
/// `AURBREAK_TEST_DIR` or `BROWSE_ARCH_TEST_DIR` env vars.
pub fn data_file() -> PathBuf {
    if let Ok(dir) =
        std::env::var("AURBREAK_TEST_DIR").or_else(|_| std::env::var("BROWSE_ARCH_TEST_DIR"))
    {
        let p = PathBuf::from(dir);
        ensure_parent(&p);
        return p.join("cache.json");
    }
    project_dir().join("cache.json")
}

fn ensure_parent(path: &std::path::Path) {
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

/// Persistent user settings configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub theme_mode: i32,        // 0 = System, 1 = Dark, 2 = Light
    pub aur_helper: String,     // "yay", "paru", "pikaur", "aura", "pacman"
    pub default_search_by: i32, // 0..7 (SearchBy enum index)
    pub search_page_size: i32,  // 25, 50, 100
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_mode: 0,
            aur_helper: "yay".to_string(),
            default_search_by: 1, // NameDesc (index 1)
            search_page_size: 50,
        }
    }
}

// ─── Search History ──────────────────────────────────────────────────────────

/// Maximum number of recent searches to persist.
const HISTORY_LIMIT: usize = 10;

/// Resolve the history file path (same dir as cache/config).
pub fn history_file() -> PathBuf {
    if let Ok(dir) =
        std::env::var("AURBREAK_TEST_DIR").or_else(|_| std::env::var("BROWSE_ARCH_TEST_DIR"))
    {
        let p = PathBuf::from(dir);
        ensure_parent(&p);
        return p.join("history.json");
    }
    project_dir().join("history.json")
}

/// Load saved search history. Returns an empty vec if the file is missing or corrupt.
pub fn load_history() -> Vec<String> {
    let path = history_file();
    if !path.exists() {
        return Vec::new();
    }
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<Vec<String>>(&bytes).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

/// Prepend `query` to history (deduplicating), cap at `HISTORY_LIMIT`, persist atomically.
pub fn push_history(query: &str) {
    let query = query.trim();
    if query.is_empty() {
        return;
    }
    let mut history = load_history();
    // Remove any existing occurrence of the same query so it bubbles to the top.
    history.retain(|q| q != query);
    history.insert(0, query.to_string());
    history.truncate(HISTORY_LIMIT);
    save_history(&history);
}

/// Persist history atomically.
pub fn save_history(history: &[String]) {
    let path = history_file();
    ensure_parent(&path);
    let bytes = match serde_json::to_vec(history) {
        Ok(b) => b,
        Err(_) => return,
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &bytes).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, &path);
}

/// Clear all saved search history.
pub fn clear_history() {
    let path = history_file();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
    }
}

/// Resolve the config file path.
pub fn config_file() -> PathBuf {
    if let Ok(dir) =
        std::env::var("AURBREAK_TEST_DIR").or_else(|_| std::env::var("BROWSE_ARCH_TEST_DIR"))
    {
        let p = PathBuf::from(dir);
        ensure_parent(&p);
        return p.join("config.json");
    }
    project_dir().join("config.json")
}

/// Load configuration from disk, falling back to defaults if not found or corrupted.
pub fn load_config() -> AppConfig {
    let path = config_file();
    if !path.exists() {
        return AppConfig::default();
    }
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

/// Persist configuration atomically to disk.
pub fn save_config(config: &AppConfig) {
    let path = config_file();
    ensure_parent(&path);
    let bytes = match serde_json::to_vec_pretty(config) {
        Ok(b) => b,
        Err(_) => return,
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &bytes).is_err() {
        return;
    }
    let _ = std::fs::rename(&tmp, &path);
}

/// Delete local cache files (cache.json and cache.json.bak).
pub fn clear_cache() -> std::io::Result<()> {
    let path = data_file();
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    let bak = path.with_extension("json.bak");
    if bak.exists() {
        let _ = std::fs::remove_file(&bak);
    }
    Ok(())
}

/// Return formatted cache file path and human-readable size.
pub fn cache_stats() -> (String, String) {
    let path = data_file();
    let path_str = path.to_string_lossy().to_string();
    if path.exists() {
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let size_str = if bytes < 1024 {
            format!("{} bytes", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else {
            format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
        };
        (path_str, format!("Cached file size: {}", size_str))
    } else {
        (path_str, "No cached data yet (0 bytes)".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_dir() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("aurbreak-test-{}-{n}", std::process::id()));
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
        // BROWSE_ARCH_TEST_DIR is process-global; serialize tests that mutate it.
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap();
        let prev = std::env::var("BROWSE_ARCH_TEST_DIR").ok();
        // SAFETY: BROWSE_ARCH_TEST_DIR is only read by this test binary, and the
        // global LOCK above serializes every mutation so there are no concurrent
        // readers of this environment variable within the process.
        unsafe {
            std::env::set_var("BROWSE_ARCH_TEST_DIR", dir);
        }
        let r = f();
        // SAFETY: same as above — serialized by LOCK, no concurrent access.
        unsafe {
            if let Some(p) = prev {
                std::env::set_var("BROWSE_ARCH_TEST_DIR", p);
            } else {
                std::env::remove_var("BROWSE_ARCH_TEST_DIR");
            }
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

    #[test]
    fn test_config_roundtrip() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            let mut cfg = load_config();
            assert_eq!(cfg, AppConfig::default());

            cfg.theme_mode = 1;
            cfg.aur_helper = "paru".to_string();
            cfg.default_search_by = 2;
            cfg.search_page_size = 25;
            save_config(&cfg);

            let loaded = load_config();
            assert_eq!(loaded, cfg);
        });
    }

    #[test]
    fn test_clear_cache() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            let path = data_file();
            ensure_parent(&path);
            fs::write(&path, b"test cache").unwrap();
            assert!(path.exists());

            let (p_str, status) = cache_stats();
            assert!(!p_str.is_empty());
            assert!(status.contains("Cached file size"));

            clear_cache().expect("clear_cache failed");
            assert!(!path.exists());

            let (_, empty_status) = cache_stats();
            assert!(empty_status.contains("No cached data"));
        });
    }

    #[test]
    fn test_history_push_and_load() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            assert!(load_history().is_empty());

            push_history("yay");
            push_history("spotify");
            push_history("telegram");

            let h = load_history();
            assert_eq!(h.len(), 3);
            assert_eq!(h[0], "telegram");
            assert_eq!(h[1], "spotify");
            assert_eq!(h[2], "yay");
        });
    }

    #[test]
    fn test_history_deduplication() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            push_history("yay");
            push_history("spotify");
            push_history("yay"); // duplicate — should bubble to top

            let h = load_history();
            assert_eq!(h.len(), 2);
            assert_eq!(h[0], "yay");
            assert_eq!(h[1], "spotify");
        });
    }

    #[test]
    fn test_history_limit() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            for i in 0..15 {
                push_history(&format!("query-{i}"));
            }
            let h = load_history();
            assert_eq!(h.len(), 10, "history must be capped at 10");
            assert_eq!(h[0], "query-14");
        });
    }

    #[test]
    fn test_history_clear() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            push_history("yay");
            push_history("paru");
            assert_eq!(load_history().len(), 2);

            clear_history();
            assert!(load_history().is_empty());
        });
    }

    #[test]
    fn test_history_ignores_blank() {
        let dir = tmp_dir();
        with_test_dir(&dir, || {
            push_history("  ");
            push_history("");
            assert!(load_history().is_empty());
        });
    }
}
