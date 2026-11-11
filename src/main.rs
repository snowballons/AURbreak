slint::include_modules!();

mod client;
mod storage;

use client::{AurPackage, DetailPackage, RssPackage, SearchBy, SearchResult};
use slint::VecModel;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread;

#[allow(clippy::enum_variant_names, clippy::large_enum_variant)]
enum BackgroundMsg {
    LastUpdatedFetched(Result<Vec<AurPackage>, String>),
    RssFetched(Result<Vec<RssPackage>, String>),
    SearchFetched(Result<SearchResult, String>),
    SuggestFetched(Result<Vec<String>, String>),
    DetailFetched(Result<DetailPackage, String>),
}

fn apply_filter(app: &AppWindow, packages: &[AurPackage], filter: &str) {
    let state = app.global::<AppState>();
    state.set_total_items_count(packages.len() as i32);
    let trimmed = filter.trim();
    if trimmed.is_empty() {
        state.set_packages(slint::ModelRc::new(packages_to_model(packages)));
        state.set_displayed_items_count(packages.len() as i32);
    } else {
        let q = trimmed.to_lowercase();
        let filtered: Vec<AurPackage> = packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q)
                    || p.description.to_lowercase().contains(&q)
                    || p.maintainer.to_lowercase().contains(&q)
            })
            .cloned()
            .collect();
        state.set_displayed_items_count(filtered.len() as i32);
        state.set_packages(slint::ModelRc::new(packages_to_model(&filtered)));
    }
}

/// Apply client-side pagination to search results.
///
/// AUR RPC v5 returns all matching results in a single response with no
/// server-side paging. We slice the full result set here so the UI shows
/// only the items for the current page.
///
/// `all_results` must be the complete unfiltered result set from the last
/// search. The current page and page size are read from `AppState`.
fn apply_search_page(app: &AppWindow, all_results: &[AurPackage]) {
    let state = app.global::<AppState>();
    let page_size = (state.get_search_page_size().max(1)) as usize;
    let page = (state.get_search_page().max(1) as usize) - 1; // 0-indexed
    let start = page * page_size;
    let end = (start + page_size).min(all_results.len());
    let slice = if start < all_results.len() {
        &all_results[start..end]
    } else {
        &[]
    };
    state.set_packages(slint::ModelRc::new(packages_to_model(slice)));
    state.set_displayed_items_count(slice.len() as i32);
    state.set_total_items_count(all_results.len() as i32);
}

pub fn save_snapshot(ui: &AppWindow, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    ui.window().set_size(slint::LogicalSize::new(1080.0, 720.0));
    ui.show()?;
    let pix = ui.window().take_snapshot()?;
    let ppm_path = format!("{output_path}.ppm");
    let mut file = std::fs::File::create(&ppm_path)?;
    use std::io::Write;
    write!(file, "P6\n{} {}\n255\n", pix.width(), pix.height())?;
    let mut bytes = Vec::with_capacity((pix.width() * pix.height() * 3) as usize);
    for pixel in pix.as_slice() {
        bytes.push(pixel.r);
        bytes.push(pixel.g);
        bytes.push(pixel.b);
    }
    file.write_all(&bytes)?;
    drop(file);
    let status = std::process::Command::new("convert")
        .args([&ppm_path, output_path])
        .status()?;
    let _ = std::fs::remove_file(ppm_path);
    if !status.success() {
        return Err(format!("convert command failed with status: {status}").into());
    }
    Ok(())
}

fn main() {
    // Headless snapshot mode for visual verification & screenshots
    if let Some(_target) = std::env::args().nth(1).filter(|a| a == "--snapshot") {
        let out_dir = std::env::args()
            .nth(2)
            .unwrap_or_else(|| "/tmp".to_string());
        let _ = std::fs::create_dir_all(&out_dir);
        println!("Generating snapshots in {out_dir}...");

        let mock_pkgs = vec![
            AurPackage {
                name: "visual-studio-code-bin".to_string(),
                version: "1.96.2-1".to_string(),
                votes: 2450,
                popularity: 42.15,
                description: "Visual Studio Code. Code editing. Redefined. Official binary package"
                    .to_string(),
                maintainer: "dpeukert".to_string(),
                updated_str: "2026-11-06 18:24 (UTC)".to_string(),
                updated_utc: None,
            },
            AurPackage {
                name: "yay".to_string(),
                version: "12.4.2-1".to_string(),
                votes: 4120,
                popularity: 68.30,
                description: "Yet another yogurt. Pacman wrapper and AUR helper written in Go"
                    .to_string(),
                maintainer: "Morganamilo".to_string(),
                updated_str: "2026-11-05 14:10 (UTC)".to_string(),
                updated_utc: None,
            },
            AurPackage {
                name: "spotify".to_string(),
                version: "1.2.53.440-1".to_string(),
                votes: 3890,
                popularity: 55.40,
                description: "A proprietary music streaming service".to_string(),
                maintainer: "gromit".to_string(),
                updated_str: "2026-11-04 09:12 (UTC)".to_string(),
                updated_utc: None,
            },
            AurPackage {
                name: "google-chrome".to_string(),
                version: "131.0.6778.85-1".to_string(),
                votes: 3120,
                popularity: 38.90,
                description: "The popular and trusted web browser by Google (stable channel)"
                    .to_string(),
                maintainer: "antonio".to_string(),
                updated_str: "2026-11-03 22:05 (UTC)".to_string(),
                updated_utc: None,
            },
            AurPackage {
                name: "rust-analyzer-bin".to_string(),
                version: "2026-11-04-1".to_string(),
                votes: 820,
                popularity: 14.50,
                description: "A modular compiler frontend for the Rust language (binary release)"
                    .to_string(),
                maintainer: "orhun".to_string(),
                updated_str: "2026-11-04 11:30 (UTC)".to_string(),
                updated_utc: None,
            },
        ];

        let ui_main = AppWindow::new().expect("Failed to create snapshot window");
        let state = ui_main.global::<AppState>();
        state.set_active_tab(0);
        state.set_last_fetched_str("just now".into());
        state.set_status_text("Loaded 5 packages • Ready".into());
        apply_filter(&ui_main, &mock_pkgs, "");

        if let Err(e) = save_snapshot(&ui_main, &format!("{out_dir}/aur-view-last-modified.png")) {
            eprintln!("Failed to save last-modified snapshot: {e}");
        }

        // Snapshot 2: Tab 2 (Search) with query and result
        let ui_search = AppWindow::new().expect("Failed to create search window");
        let state_search = ui_search.global::<AppState>();
        state_search.set_active_tab(2);
        state_search.set_search_query("telegram".into());
        state_search.set_search_result_count(1);
        state_search.set_search_page(1);
        state_search.set_search_total_pages(1);
        let search_pkgs = vec![AurPackage {
            name: "telegram-a-appimage".to_string(),
            version: "10.9.48-1".to_string(),
            votes: 18,
            popularity: 0.85,
            description: "Official Telegram Web client version A AppImage version".to_string(),
            maintainer: "alesar1".to_string(),
            updated_str: "2025-04-05 20:08 (UTC)".to_string(),
            updated_utc: None,
        }];
        apply_filter(&ui_search, &search_pkgs, "");
        state_search.set_status_text("Found 1 package.".into());
        if let Err(e) = save_snapshot(&ui_search, &format!("{out_dir}/aur-view-search.png")) {
            eprintln!("Failed to save search snapshot: {e}");
        }

        // Snapshot 3: Detail pane open side-by-side with search results
        let ui_detail = AppWindow::new().expect("Failed to create detail window");
        let state_detail = ui_detail.global::<AppState>();
        state_detail.set_active_tab(2);
        state_detail.set_search_query("telegram".into());
        state_detail.set_search_result_count(1);
        state_detail.set_search_page(1);
        state_detail.set_search_total_pages(1);
        apply_filter(&ui_detail, &search_pkgs, "");
        state_detail.set_status_text("Found 1 package.".into());
        state_detail.set_show_detail(true);
        state_detail.set_detail_name("telegram-a-appimage".into());
        state_detail.set_detail_version("10.9.48-1".into());
        state_detail.set_detail_description("Official Telegram Web client version A AppImage version. Provides a seamless desktop experience for Telegram Web A with native desktop notifications, media keys, and system tray integration.".into());
        state_detail.set_detail_url("https://web.telegram.org/a/".into());
        state_detail.set_detail_maintainer("alesar1".into());
        state_detail.set_detail_submitter("alesar1".into());
        state_detail.set_detail_votes(18);
        state_detail.set_detail_popularity(0.85);
        state_detail.set_detail_license("GPL-3.0-only".into());
        state_detail.set_detail_last_modified("2025-04-05 20:08 UTC".into());
        state_detail.set_detail_first_submitted("2024-01-15 12:00 UTC".into());
        state_detail.set_detail_depends(slint::ModelRc::new(slint::VecModel::from(vec![
            "zlib".into(),
            "glibc".into(),
            "fuse2".into(),
        ])));
        state_detail.set_detail_make_depends(slint::ModelRc::new(slint::VecModel::from(vec![
            "squashfs-tools".into(),
        ])));
        state_detail.set_detail_opt_depends(slint::ModelRc::new(slint::VecModel::from(vec![
            "libnotify: Desktop notifications support".into(),
        ])));
        if let Err(e) = save_snapshot(&ui_detail, &format!("{out_dir}/aur-view-detail.png")) {
            eprintln!("Failed to save detail snapshot: {e}");
        }

        // Snapshot 4: Dark mode preview
        let ui_dark = AppWindow::new().expect("Failed to create dark mode window");
        ui_dark.global::<Theme>().set_force_dark(true);
        let state_dark = ui_dark.global::<AppState>();
        state_dark.set_active_tab(0);
        state_dark.set_last_fetched_str("just now".into());
        state_dark.set_status_text("Loaded 5 packages • Ready".into());
        apply_filter(&ui_dark, &mock_pkgs, "");
        if let Err(e) = save_snapshot(
            &ui_dark,
            &format!("{out_dir}/aur-view-last-modified-dark.png"),
        ) {
            eprintln!("Failed to save dark snapshot: {e}");
        }

        // Snapshot 5: Dark mode with detail inspector
        let ui_detail_dark = AppWindow::new().expect("Failed to create dark detail window");
        ui_detail_dark.global::<Theme>().set_force_dark(true);
        let state_detail_dark = ui_detail_dark.global::<AppState>();
        state_detail_dark.set_active_tab(2);
        state_detail_dark.set_search_query("telegram".into());
        state_detail_dark.set_search_result_count(1);
        state_detail_dark.set_search_page(1);
        state_detail_dark.set_search_total_pages(1);
        apply_filter(&ui_detail_dark, &search_pkgs, "");
        state_detail_dark.set_status_text("Found 1 package.".into());
        state_detail_dark.set_show_detail(true);
        state_detail_dark.set_detail_name("telegram-a-appimage".into());
        state_detail_dark.set_detail_version("10.9.48-1".into());
        state_detail_dark.set_detail_description("Official Telegram Web client version A AppImage version. Provides a seamless desktop experience for Telegram Web A with native desktop notifications, media keys, and system tray integration.".into());
        state_detail_dark.set_detail_url("https://web.telegram.org/a/".into());
        state_detail_dark.set_detail_maintainer("alesar1".into());
        state_detail_dark.set_detail_submitter("alesar1".into());
        state_detail_dark.set_detail_votes(18);
        state_detail_dark.set_detail_popularity(0.85);
        state_detail_dark.set_detail_license("GPL-3.0-only".into());
        state_detail_dark.set_detail_last_modified("2025-04-05 20:08 UTC".into());
        state_detail_dark.set_detail_first_submitted("2024-01-15 12:00 UTC".into());
        state_detail_dark.set_detail_depends(slint::ModelRc::new(slint::VecModel::from(vec![
            "zlib".into(),
            "glibc".into(),
            "fuse2".into(),
        ])));
        state_detail_dark.set_detail_make_depends(slint::ModelRc::new(slint::VecModel::from(
            vec!["squashfs-tools".into()],
        )));
        state_detail_dark.set_detail_opt_depends(slint::ModelRc::new(slint::VecModel::from(vec![
            "libnotify: Desktop notifications support".into(),
        ])));
        if let Err(e) = save_snapshot(
            &ui_detail_dark,
            &format!("{out_dir}/aur-view-detail-dark.png"),
        ) {
            eprintln!("Failed to save dark detail snapshot: {e}");
        }

        // Snapshot 6: Settings View (Light)
        let ui_settings = AppWindow::new().expect("Failed to create settings window");
        let state_settings = ui_settings.global::<AppState>();
        state_settings.set_show_settings(true);
        state_settings.set_cache_path("~/.local/share/aurbreak/cache.json".into());
        state_settings.set_cache_size_str("Cached file size: 142.5 KB".into());
        state_settings.set_aur_helper("yay".into());
        state_settings.set_default_search_by(1);
        state_settings.set_search_page_size(50);
        state_settings.set_theme_mode(0);
        if let Err(e) = save_snapshot(&ui_settings, &format!("{out_dir}/aur-view-settings.png")) {
            eprintln!("Failed to save settings snapshot: {e}");
        }

        // Snapshot 7: Settings View (Dark)
        let ui_settings_dark = AppWindow::new().expect("Failed to create dark settings window");
        ui_settings_dark.global::<Theme>().set_force_dark(true);
        let state_settings_dark = ui_settings_dark.global::<AppState>();
        state_settings_dark.set_show_settings(true);
        state_settings_dark.set_cache_path("~/.local/share/aurbreak/cache.json".into());
        state_settings_dark.set_cache_size_str("Cached file size: 142.5 KB".into());
        state_settings_dark.set_aur_helper("yay".into());
        state_settings_dark.set_default_search_by(1);
        state_settings_dark.set_search_page_size(50);
        state_settings_dark.set_theme_mode(1);
        if let Err(e) = save_snapshot(
            &ui_settings_dark,
            &format!("{out_dir}/aur-view-settings-dark.png"),
        ) {
            eprintln!("Failed to save dark settings snapshot: {e}");
        }

        println!("Snapshots generated successfully!");
        return;
    }

    let ui = AppWindow::new().expect("Failed to create Slint window");
    let (tx, rx) = mpsc::channel::<BackgroundMsg>();
    let current_pkgs = Rc::new(RefCell::new(Vec::<AurPackage>::new()));

    // Load saved settings & cache stats
    let config = storage::load_config();
    {
        let state = ui.global::<AppState>();
        let (cache_path, cache_size) = storage::cache_stats();
        state.set_cache_path(cache_path.into());
        state.set_cache_size_str(cache_size.into());
        state.set_theme_mode(config.theme_mode);
        state.set_aur_helper(config.aur_helper.into());
        state.set_default_search_by(config.default_search_by);
        state.set_search_by(config.default_search_by);
        state.set_search_page_size(config.search_page_size);
        ui.global::<Theme>().set_theme_mode(config.theme_mode);

        // Restore persisted search history into the UI model.
        let hist: Vec<slint::SharedString> = storage::load_history()
            .into_iter()
            .map(|s| s.into())
            .collect();
        state.set_search_history(slint::ModelRc::new(slint::VecModel::from(hist)));

        // Inject the build-time version string so the About panel in Settings
        // always reflects the actual binary version rather than a hardcoded default.
        state.set_app_version(env!("CARGO_PKG_VERSION").into());
    }

    // Offline-first: paint from the local cache immediately, then refresh in
    // the background if the cache is empty or stale (older than 6 hours).
    let cached = match storage::load() {
        storage::LoadOutcome::Loaded(loaded) => {
            let age = chrono::Utc::now().signed_duration_since(loaded.fetched_at);
            let state = ui.global::<AppState>();
            state.set_last_fetched_str(format!("cached {} ago", format_duration(age)).into());
            *current_pkgs.borrow_mut() = loaded.packages.clone();
            apply_filter(&ui, &loaded.packages, "");
            Some((loaded.packages, age))
        }
        _ => None,
    };

    match cached {
        None => {
            // No cache on disk yet — fetch immediately.
            trigger_last_updated_fetch(&ui, &tx);
        }
        Some((_, age)) if age > chrono::Duration::hours(6) => {
            // Cache is stale; refresh in the background while the stale data
            // remains visible so the app feels instant.
            trigger_last_updated_fetch(&ui, &tx);
        }
        Some(_) => {
            // Cache is fresh — nothing to do until the user manually refreshes.
        }
    }

    // Filter callback handler
    let ui_weak = ui.as_weak();
    let current_pkgs_filter = current_pkgs.clone();
    ui.global::<AppLogic>().on_filter_changed(move |filter| {
        if let Some(app) = ui_weak.upgrade() {
            let pkgs = current_pkgs_filter.borrow();
            apply_filter(&app, &pkgs, &filter);
        }
    });

    // Callback handlers from AppLogic.
    let ui_weak = ui.as_weak();
    let tx_refresh = tx.clone();
    ui.global::<AppLogic>().on_refresh_requested(move || {
        if let Some(app) = ui_weak.upgrade() {
            let tab = app.global::<AppState>().get_active_tab();
            if tab == 0 {
                trigger_last_updated_fetch(&app, &tx_refresh);
            } else if tab == 1 {
                trigger_rss_fetch(&app, &tx_refresh);
            } else if tab == 2 {
                // Re-run current search
                let state = app.global::<AppState>();
                let query = state.get_search_query().to_string();
                if !query.is_empty() {
                    let search_by_idx = state.get_search_by() as usize;
                    let by = SearchBy::all()
                        .get(search_by_idx)
                        .copied()
                        .unwrap_or(SearchBy::NameDesc);
                    state.set_search_loading(true);
                    let tx = tx_refresh.clone();
                    thread::spawn(move || {
                        let res =
                            client::search_packages(&query, Some(by)).map_err(|e| e.to_string());
                        let _ = tx.send(BackgroundMsg::SearchFetched(res));
                    });
                }
            }
        }
    });

    let ui_weak = ui.as_weak();
    let tx_tab = tx.clone();
    let current_pkgs_tab = current_pkgs.clone();
    ui.global::<AppLogic>().on_tab_changed(move |new_tab| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_show_settings(false);
            state.set_active_tab(new_tab);
            state.set_search_filter("".into());
            // Trigger fade-in: drop opacity to 0 then immediately restore to 1
            // so the animate declaration plays the transition.
            state.set_tab_opacity(0.0);
            state.set_tab_opacity(1.0);
            if new_tab == 0 {
                // Guard: skip if a fetch is already in flight to avoid stacking threads.
                if !state.get_loading() {
                    trigger_last_updated_fetch(&app, &tx_tab);
                }
            } else if new_tab == 1 {
                if !state.get_loading() {
                    trigger_rss_fetch(&app, &tx_tab);
                }
            } else if new_tab == 2 {
                // Search tab - don't auto-fetch, wait for user query
                current_pkgs_tab.borrow_mut().clear();
                state.set_packages(slint::ModelRc::new(VecModel::default()));
                state.set_total_items_count(0);
                state.set_displayed_items_count(0);
                state.set_status_text("Enter a search query above to search AUR".into());
            }
        }
    });

    ui.global::<AppLogic>().on_open_pkg_page(move |pkg_name| {
        let url = format!("https://aur.archlinux.org/packages/{pkg_name}");
        // open::that() blocks on Linux until xdg-open exits (which can be
        // until the browser window closes). Use that_in_background() so the
        // UI thread is never stalled.
        let _ = open::that_in_background(&url);
    });

    ui.global::<AppLogic>().on_open_url(move |raw_url| {
        let s = raw_url.to_string();
        // Only open http/https URLs — reject file://, custom protocol handlers,
        // or anything a malicious AUR package metadata field could inject.
        if s.starts_with("https://") || s.starts_with("http://") {
            let _ = open::that_in_background(&s);
        }
    });

    // Settings callbacks
    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_toggle_settings(move || {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            let curr = state.get_show_settings();
            state.set_show_settings(!curr);
            if !curr {
                let (p, s) = storage::cache_stats();
                state.set_cache_path(p.into());
                state.set_cache_size_str(s.into());
            }
        }
    });

    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_close_settings(move || {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_show_settings(false);
        }
    });

    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_theme_mode_changed(move |mode| {
        if let Some(app) = ui_weak.upgrade() {
            app.global::<Theme>().set_theme_mode(mode);
            let mut cfg = storage::load_config();
            cfg.theme_mode = mode;
            storage::save_config(&cfg);
        }
    });

    ui.global::<AppLogic>()
        .on_aur_helper_changed(move |helper| {
            let mut cfg = storage::load_config();
            cfg.aur_helper = helper.to_string();
            storage::save_config(&cfg);
        });

    ui.global::<AppLogic>()
        .on_default_search_by_changed(move |by| {
            let mut cfg = storage::load_config();
            cfg.default_search_by = by;
            storage::save_config(&cfg);
        });

    let ui_weak = ui.as_weak();
    let current_pkgs_pgsize = current_pkgs.clone();
    ui.global::<AppLogic>()
        .on_search_page_size_changed(move |size| {
            if let Some(app) = ui_weak.upgrade() {
                let state = app.global::<AppState>();
                let count = state.get_search_result_count();
                if count > 0 && size > 0 {
                    // Recalculate total pages and reset to page 1.
                    state.set_search_page(1);
                    state.set_search_total_pages(
                        ((count as f64) / (size as f64)).ceil().max(1.0) as i32
                    );
                    // Re-slice the existing results for the new page size.
                    let pkgs = current_pkgs_pgsize.borrow();
                    apply_search_page(&app, &pkgs);
                }
                let mut cfg = storage::load_config();
                cfg.search_page_size = size;
                storage::save_config(&cfg);
            }
        });

    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_clear_cache_requested(move || {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            let _ = storage::clear_cache();
            let (p, s) = storage::cache_stats();
            state.set_cache_path(p.into());
            state.set_cache_size_str(s.into());
            state.set_status_text("Local cache cleared successfully.".into());
        }
    });

    // Search callback
    let ui_weak = ui.as_weak();
    let tx_search = tx.clone();
    ui.global::<AppLogic>()
        .on_search_requested(move |query, by, _page| {
            if let Some(app) = ui_weak.upgrade() {
                let state = app.global::<AppState>();
                let q = query.to_string();
                state.set_search_loading(true);
                state.set_search_error("".into()); // clear any prior error
                state.set_suggestions(slint::ModelRc::new(VecModel::default())); // dismiss dropdown
                state.set_status_text(format!("Searching for \"{}\"…", q).into());

                // Persist this query to search history immediately.
                storage::push_history(&q);
                // Reload history into state so chips update.
                let hist: Vec<slint::SharedString> = storage::load_history()
                    .into_iter()
                    .map(|s| s.into())
                    .collect();
                state.set_search_history(slint::ModelRc::new(slint::VecModel::from(hist)));

                let tx = tx_search.clone();
                let by = SearchBy::all()
                    .get(by as usize)
                    .copied()
                    .unwrap_or(SearchBy::NameDesc);
                thread::spawn(move || {
                    let res = client::search_packages(&q, Some(by)).map_err(|e| e.to_string());
                    let _ = tx.send(BackgroundMsg::SearchFetched(res));
                });
            }
        });

    // Suggest callback — 300ms debounce so we don't fire a request on every
    // keystroke. We hold a single-shot timer in an Rc<RefCell> and restart it
    // on each call; only the final keystroke within the debounce window fires.
    let ui_weak = ui.as_weak();
    let tx_suggest = tx.clone();
    let debounce_timer = Rc::new(RefCell::new(slint::Timer::default()));
    ui.global::<AppLogic>().on_get_suggestions(move |prefix| {
        let prefix_str = prefix.to_string();
        if prefix_str.len() < 2 {
            // Clear stale suggestions immediately when the input is too short.
            if let Some(app) = ui_weak.upgrade() {
                app.global::<AppState>()
                    .set_suggestions(slint::ModelRc::new(VecModel::default()));
            }
            return;
        }
        let tx = tx_suggest.clone();
        let ui_weak2 = ui_weak.clone();
        let timer_ref = debounce_timer.clone();
        // Restart the debounce window.
        let new_timer = slint::Timer::default();
        new_timer.start(
            slint::TimerMode::SingleShot,
            std::time::Duration::from_millis(300),
            move || {
                // If the UI is gone by the time the timer fires, skip.
                if ui_weak2.upgrade().is_none() {
                    return;
                }
                let res_tx = tx.clone();
                let p = prefix_str.clone();
                thread::spawn(move || {
                    let res = client::suggest_packages(&p).map_err(|e| e.to_string());
                    let _ = res_tx.send(BackgroundMsg::SuggestFetched(res));
                });
            },
        );
        *timer_ref.borrow_mut() = new_timer;
    });

    // Search page change callback — re-slice the already-fetched results for
    // the requested page. No new network request needed: AUR RPC v5 returns
    // all results in a single response.
    let ui_weak = ui.as_weak();
    let current_pkgs_page = current_pkgs.clone();
    ui.global::<AppLogic>().on_search_page_changed(move |page| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            if state.get_search_query().is_empty() {
                return;
            }
            state.set_search_page(page);
            let pkgs = current_pkgs_page.borrow();
            apply_search_page(&app, &pkgs);
        }
    });

    // Search by changed callback
    let ui_weak = ui.as_weak();
    let tx_by = tx.clone();
    ui.global::<AppLogic>().on_search_by_changed(move |by| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            let query = state.get_search_query().to_string();
            if query.is_empty() {
                return;
            }
            let by = SearchBy::all()
                .get(by as usize)
                .copied()
                .unwrap_or(SearchBy::NameDesc);
            state.set_search_loading(true);
            let tx = tx_by.clone();
            thread::spawn(move || {
                let res = client::search_packages(&query, Some(by)).map_err(|e| e.to_string());
                let _ = tx.send(BackgroundMsg::SearchFetched(res));
            });
        }
    });

    // Show package detail callback
    let ui_weak = ui.as_weak();
    let tx_detail = tx.clone();
    ui.global::<AppLogic>().on_show_package_detail(move |name| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_show_detail(true);
            state.set_detail_name(name.clone());
            state.set_detail_version("…".into());
            state.set_detail_description("Loading details from aur.archlinux.org…".into());
            state.set_detail_maintainer("".into());
            state.set_detail_submitter("".into());
            state.set_detail_votes(0);
            state.set_detail_popularity(0.0);
            state.set_detail_url("".into());
            state.set_detail_license("".into());
            state.set_detail_depends(slint::ModelRc::new(VecModel::default()));
            state.set_detail_make_depends(slint::ModelRc::new(VecModel::default()));
            state.set_detail_opt_depends(slint::ModelRc::new(VecModel::default()));
            state.set_detail_check_depends(slint::ModelRc::new(VecModel::default()));
            state.set_detail_provides(slint::ModelRc::new(VecModel::default()));
            state.set_detail_conflicts(slint::ModelRc::new(VecModel::default()));
            state.set_detail_replaces(slint::ModelRc::new(VecModel::default()));
            state.set_detail_groups(slint::ModelRc::new(VecModel::default()));
            state.set_detail_keywords(slint::ModelRc::new(VecModel::default()));
            state.set_detail_co_maintainers(slint::ModelRc::new(VecModel::default()));
            state.set_detail_first_submitted("".into());
            state.set_detail_last_modified("".into());
            state.set_detail_out_of_date("".into());

            let name_str = name.to_string();
            let tx = tx_detail.clone();
            thread::spawn(move || {
                let res = client::get_package_info(&name_str)
                    .map_err(|e| e.to_string())
                    .map(DetailPackage::from_detailed);
                let _ = tx.send(BackgroundMsg::DetailFetched(res));
            });
        }
    });

    // Close detail callback
    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_close_detail(move || {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_show_detail(false);
        }
    });

    // Clear search history callback
    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_clear_search_history(move || {
        storage::clear_history();
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_search_history(slint::ModelRc::new(VecModel::default()));
        }
    });

    // Retry last search callback — re-runs the current query with the current
    // search-by setting. Used by the error state retry button.
    let ui_weak = ui.as_weak();
    let tx_retry = tx.clone();
    ui.global::<AppLogic>().on_retry_search(move || {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            let query = state.get_search_query().to_string();
            if query.is_empty() {
                return;
            }
            let by = SearchBy::all()
                .get(state.get_search_by() as usize)
                .copied()
                .unwrap_or(SearchBy::NameDesc);
            state.set_search_loading(true);
            state.set_search_error("".into());
            state.set_status_text(format!("Retrying search for \"{}\"…", query).into());
            let tx = tx_retry.clone();
            thread::spawn(move || {
                let res = client::search_packages(&query, Some(by)).map_err(|e| e.to_string());
                let _ = tx.send(BackgroundMsg::SearchFetched(res));
            });
        }
    });

    // Dismiss suggestions (e.g. Esc pressed in search input)
    let ui_weak = ui.as_weak();
    ui.global::<AppLogic>().on_dismiss_suggestions(move || {
        if let Some(app) = ui_weak.upgrade() {
            app.global::<AppState>()
                .set_suggestions(slint::ModelRc::new(VecModel::default()));
        }
    });

    // Event loop poll for background thread messages.
    let ui_weak = ui.as_weak();
    let current_pkgs_event = current_pkgs.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            // Advance spinner frame while any loading is active.
            if let Some(app) = ui_weak.upgrade() {
                let state = app.global::<AppState>();
                if state.get_search_loading() || state.get_loading() {
                    state.set_spinner_frame((state.get_spinner_frame() + 1) % 8);
                }
            }
            while let Ok(msg) = rx.try_recv() {
                if let Some(app) = ui_weak.upgrade() {
                    let state = app.global::<AppState>();
                    match msg {
                        BackgroundMsg::LastUpdatedFetched(res) => {
                            state.set_loading(false);

                            match res {
                                Ok(pkgs) => {
                                    let now = chrono::Utc::now();
                                    storage::save(&pkgs, now);
                                    *current_pkgs_event.borrow_mut() = pkgs.clone();
                                    let filter = state.get_search_filter().to_string();
                                    apply_filter(&app, &pkgs, &filter);
                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Fetched {} packages successfully.", pkgs.len())
                                            .into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("Fetch failed: {}", err).into());
                                }
                            }
                        }
                        BackgroundMsg::RssFetched(res) => {
                            state.set_loading(false);

                            match res {
                                Ok(rss_pkgs) => {
                                    let aur_pkgs: Vec<AurPackage> = rss_pkgs
                                        .into_iter()
                                        .map(|r| AurPackage {
                                            name: r.name,
                                            version: "new".to_string(),
                                            votes: 0,
                                            popularity: 0.0,
                                            description: r.description,
                                            maintainer: "".to_string(),
                                            updated_str: r.pub_date,
                                            updated_utc: None,
                                        })
                                        .collect();

                                    *current_pkgs_event.borrow_mut() = aur_pkgs.clone();
                                    let filter = state.get_search_filter().to_string();
                                    apply_filter(&app, &aur_pkgs, &filter);
                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Fetched {} RSS submission items.", aur_pkgs.len())
                                            .into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(
                                        format!("RSS fetch failed: {}", err).into(),
                                    );
                                }
                            }
                        }
                        BackgroundMsg::SearchFetched(res) => {
                            state.set_loading(false);
                            state.set_search_loading(false);

                            match res {
                                Ok(search_result) => {
                                    let total = search_result.resultcount;
                                    let aur_pkgs: Vec<AurPackage> = search_result
                                        .results
                                        .into_iter()
                                        .map(|p| AurPackage {
                                            name: p.name,
                                            version: p.version,
                                            votes: p.num_votes,
                                            popularity: p.popularity,
                                            description: p.description.unwrap_or_default(),
                                            maintainer: p.maintainer.unwrap_or_default(),
                                            updated_str: {
                                                let dt = chrono::DateTime::from_timestamp(
                                                    p.last_modified,
                                                    0,
                                                );
                                                dt.map(|d| {
                                                    d.format("%Y-%m-%d %H:%M (UTC)").to_string()
                                                })
                                                .unwrap_or_default()
                                            },
                                            updated_utc: chrono::DateTime::from_timestamp(
                                                p.last_modified,
                                                0,
                                            ),
                                        })
                                        .collect();

                                    *current_pkgs_event.borrow_mut() = aur_pkgs.clone();

                                    // Reset to page 1 on every new search result, then slice.
                                    state.set_search_page(1);
                                    state.set_search_error("".into()); // clear any prior error
                                    state.set_search_result_count(total);
                                    let page_size = state.get_search_page_size().max(1);
                                    let actual_count = aur_pkgs.len() as i32;
                                    state.set_search_total_pages(
                                        ((actual_count as f64) / (page_size as f64)).ceil().max(1.0)
                                            as i32,
                                    );
                                    // Show only the first page of results.
                                    apply_search_page(&app, &aur_pkgs);

                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Found {} packages.", total).into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("Search failed: {}", err).into());
                                    state.set_search_error(err.into());
                                }
                            }
                        }
                        BackgroundMsg::SuggestFetched(res) => {
                            if let Ok(suggestions) = res {
                                let shared: Vec<slint::SharedString> =
                                    suggestions.into_iter().map(|s| s.into()).collect();
                                state.set_suggestions(slint::ModelRc::new(slint::VecModel::from(
                                    shared,
                                )));
                            }
                        }
                        BackgroundMsg::DetailFetched(res) => {
                            state.set_loading(false);

                            match res {
                                Ok(detail) => {
                                    state.set_show_detail(true);
                                    state.set_detail_name(detail.name.into());
                                    state.set_detail_version(detail.version.into());
                                    state.set_detail_description(detail.description.into());
                                    state.set_detail_maintainer(detail.maintainer.into());
                                    state.set_detail_votes(detail.votes);
                                    state.set_detail_popularity(detail.popularity as f32);
                                    state.set_detail_url(detail.url.into());
                                    state.set_detail_license(detail.license.join(", ").into());

                                    let depends: Vec<slint::SharedString> =
                                        detail.depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_depends(slint::ModelRc::new(
                                        slint::VecModel::from(depends),
                                    ));

                                    let make_depends: Vec<slint::SharedString> =
                                        detail.make_depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_make_depends(slint::ModelRc::new(
                                        slint::VecModel::from(make_depends),
                                    ));

                                    let opt_depends: Vec<slint::SharedString> =
                                        detail.opt_depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_opt_depends(slint::ModelRc::new(
                                        slint::VecModel::from(opt_depends),
                                    ));

                                    let check_depends: Vec<slint::SharedString> = detail
                                        .check_depends
                                        .into_iter()
                                        .map(|s| s.into())
                                        .collect();
                                    state.set_detail_check_depends(slint::ModelRc::new(
                                        slint::VecModel::from(check_depends),
                                    ));

                                    let provides: Vec<slint::SharedString> =
                                        detail.provides.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_provides(slint::ModelRc::new(
                                        slint::VecModel::from(provides),
                                    ));

                                    let conflicts: Vec<slint::SharedString> =
                                        detail.conflicts.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_conflicts(slint::ModelRc::new(
                                        slint::VecModel::from(conflicts),
                                    ));

                                    let replaces: Vec<slint::SharedString> =
                                        detail.replaces.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_replaces(slint::ModelRc::new(
                                        slint::VecModel::from(replaces),
                                    ));

                                    let groups: Vec<slint::SharedString> =
                                        detail.groups.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_groups(slint::ModelRc::new(
                                        slint::VecModel::from(groups),
                                    ));

                                    let keywords: Vec<slint::SharedString> =
                                        detail.keywords.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_keywords(slint::ModelRc::new(
                                        slint::VecModel::from(keywords),
                                    ));

                                    let co_maintainers: Vec<slint::SharedString> = detail
                                        .co_maintainers
                                        .into_iter()
                                        .map(|s| s.into())
                                        .collect();
                                    state.set_detail_co_maintainers(slint::ModelRc::new(
                                        slint::VecModel::from(co_maintainers),
                                    ));
                                    state.set_detail_submitter(detail.submitter.into());
                                    state.set_detail_first_submitted(
                                        chrono::DateTime::from_timestamp(detail.first_submitted, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default()
                                            .into(),
                                    );
                                    state.set_detail_last_modified(
                                        chrono::DateTime::from_timestamp(detail.last_modified, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default()
                                            .into(),
                                    );
                                    state.set_detail_out_of_date(
                                        detail
                                            .out_of_date
                                            .and_then(|ts| chrono::DateTime::from_timestamp(ts, 0))
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default()
                                            .into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(
                                        format!("Detail fetch failed: {}", err).into(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        },
    );

    ui.run().expect("Failed to run Slint app");
}

fn trigger_last_updated_fetch(app: &AppWindow, tx: &mpsc::Sender<BackgroundMsg>) {
    let state = app.global::<AppState>();
    state.set_loading(true);
    state.set_status_text("Fetching latest modified packages from aur.archlinux.org…".into());

    let tx = tx.clone();
    thread::spawn(move || {
        let res = client::fetch_last_updated().map_err(|e| e.to_string());
        let _ = tx.send(BackgroundMsg::LastUpdatedFetched(res));
    });
}

fn trigger_rss_fetch(app: &AppWindow, tx: &mpsc::Sender<BackgroundMsg>) {
    let state = app.global::<AppState>();
    state.set_loading(true);
    state.set_status_text("Fetching newest RSS submissions from aur.archlinux.org…".into());

    let tx = tx.clone();
    thread::spawn(move || {
        let res = client::fetch_rss_newest().map_err(|e| e.to_string());
        let _ = tx.send(BackgroundMsg::RssFetched(res));
    });
}

fn packages_to_model(pkgs: &[AurPackage]) -> VecModel<PackageItem> {
    pkgs.iter()
        .map(|p| PackageItem {
            name: p.name.clone().into(),
            version: p.version.clone().into(),
            votes: p.votes,
            popularity: if p.popularity > 0.0 {
                format!("{:.2}", p.popularity).into()
            } else {
                "".to_string().into()
            },
            description: p.description.clone().into(),
            maintainer: p.maintainer.clone().into(),
            updated: p.updated_str.clone().into(),
        })
        .collect()
}

fn format_duration(dur: chrono::Duration) -> String {
    let mins = dur.num_minutes();
    if mins < 1 {
        "just now".to_string()
    } else if mins < 60 {
        format!("{mins}m")
    } else {
        format!("{}h", dur.num_hours())
    }
}
