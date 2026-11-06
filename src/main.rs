slint::include_modules!();

mod client;
mod storage;

use client::{AurPackage, DetailPackage, RssPackage, SearchBy, SearchResult};
use slint::{VecModel};
use std::sync::mpsc;
use std::thread;

enum BackgroundMsg {
    LastUpdatedFetched(Result<Vec<AurPackage>, String>),
    RssFetched(Result<Vec<RssPackage>, String>),
    SearchFetched(Result<SearchResult, String>),
    SuggestFetched(Result<Vec<String>, String>),
    DetailFetched(Result<DetailPackage, String>),
}

fn main() {
    let ui = AppWindow::new().expect("Failed to create Slint window");
    let (tx, rx) = mpsc::channel::<BackgroundMsg>();

    // Offline-first: paint from the local cache immediately, then refresh in
    // the background if the cache is empty or stale (older than 6 hours).
    let cached = match storage::load() {
        storage::LoadOutcome::Loaded(loaded) => {
            let age = chrono::Utc::now().signed_duration_since(loaded.fetched_at);
            let state = ui.global::<AppState>();
            state.set_packages(slint::ModelRc::new(packages_to_model(&loaded.packages)));
            state.set_last_fetched_str(format!("cached {} ago", format_duration(age)).into());
            Some(loaded.packages)
        }
        _ => None,
    };

    if cached.is_none() {
        trigger_last_updated_fetch(&ui, &tx);
    }

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
                    let by = SearchBy::all().get(search_by_idx).copied().unwrap_or(SearchBy::NameDesc);
                    state.set_search_loading(true);
                    let tx = tx_refresh.clone();
                    thread::spawn(move || {
                        let res = client::search_packages(&query, Some(by)).map_err(|e| e.to_string());
                        let _ = tx.send(BackgroundMsg::SearchFetched(res));
                    });
                }
            }
        }
    });

    let ui_weak = ui.as_weak();
    let tx_tab = tx.clone();
    ui.global::<AppLogic>().on_tab_changed(move |new_tab| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_active_tab(new_tab);
            if new_tab == 0 {
                trigger_last_updated_fetch(&app, &tx_tab);
            } else if new_tab == 1 {
                trigger_rss_fetch(&app, &tx_tab);
            } else if new_tab == 2 {
                // Search tab - don't auto-fetch, wait for user query
                state.set_packages(slint::ModelRc::new(VecModel::default()));
                state.set_status_text("Enter a search query above".into());
            }
        }
    });

    ui.global::<AppLogic>().on_open_pkg_page(move |pkg_name| {
        let url = format!("https://aur.archlinux.org/packages/{pkg_name}");
        let _ = open::that(&url);
    });

    // Search callback with debouncing (handled in UI, but we also handle here)
    let ui_weak = ui.as_weak();
    let tx_search = tx.clone();
    ui.global::<AppLogic>().on_search_requested(move |query, by, _page| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            state.set_search_loading(true);
            state.set_status_text(format!("Searching for \"{}\"…", query).into());

            let tx = tx_search.clone();
            let query = query.to_string();
            let by = SearchBy::all().get(by as usize).copied().unwrap_or(SearchBy::NameDesc);
            thread::spawn(move || {
                let res = client::search_packages(&query, Some(by)).map_err(|e| e.to_string());
                let _ = tx.send(BackgroundMsg::SearchFetched(res));
            });
        }
    });

    // Suggest callback for autocomplete
    let ui_weak = ui.as_weak();
    let tx_suggest = tx.clone();
    ui.global::<AppLogic>().on_get_suggestions(move |prefix| {
        if let Some(_app) = ui_weak.upgrade() {
            let prefix = prefix.to_string();
            if prefix.len() < 2 {
                return;
            }
            let tx = tx_suggest.clone();
            thread::spawn(move || {
                let res = client::suggest_packages(&prefix).map_err(|e| e.to_string());
                let _ = tx.send(BackgroundMsg::SuggestFetched(res));
            });
        }
    });

    // Search page change callback
    let ui_weak = ui.as_weak();
    let tx_page = tx.clone();
    ui.global::<AppLogic>().on_search_page_changed(move |_page| {
        if let Some(app) = ui_weak.upgrade() {
            let state = app.global::<AppState>();
            let query = state.get_search_query().to_string();
            if query.is_empty() {
                return;
            }
            let by = SearchBy::all().get(state.get_search_by() as usize).copied().unwrap_or(SearchBy::NameDesc);
            state.set_search_loading(true);
            let tx = tx_page.clone();
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
        if let Some(_app) = ui_weak.upgrade() {
            let name = name.to_string();
            let tx = tx_detail.clone();
            thread::spawn(move || {
                let res = client::get_package_info(&name).map_err(|e| e.to_string()).map(DetailPackage::from_detailed);
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

    // Event loop poll for background thread messages.
    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(100),
        move || {
            while let Ok(msg) = rx.try_recv() {
                if let Some(app) = ui_weak.upgrade() {
                    let state = app.global::<AppState>();
                    match msg {
                        BackgroundMsg::LastUpdatedFetched(res) => {
                            state.set_loading(false);
                            state.set_status_text("".into());

                            match res {
                                Ok(pkgs) => {
                                    let now = chrono::Utc::now();
                                    storage::save(&pkgs, now);
                                    state.set_packages(slint::ModelRc::new(packages_to_model(&pkgs)));
                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Fetched {} packages successfully.", pkgs.len()).into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("Fetch failed: {}", err).into());
                                }
                            }
                        }
                        BackgroundMsg::RssFetched(res) => {
                            state.set_loading(false);
                            state.set_status_text("".into());

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

                                    state.set_packages(slint::ModelRc::new(packages_to_model(&aur_pkgs)));
                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Fetched {} RSS submission items.", aur_pkgs.len()).into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("RSS fetch failed: {}", err).into());
                                }
                            }
                        }
                        BackgroundMsg::SearchFetched(res) => {
                            state.set_loading(false);
                            state.set_search_loading(false);
                            state.set_status_text("".into());

                            match res {
                                Ok(search_result) => {
                                    // Convert PackageBasic to AurPackage for display
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
                                                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(p.last_modified, 0);
                                                dt.map(|d| d.format("%Y-%m-%d %H:%M (UTC)").to_string())
                                                    .unwrap_or_default()
                                            },
                                            updated_utc: chrono::DateTime::<chrono::Utc>::from_timestamp(p.last_modified, 0),
                                        })
                                        .collect();

                                    state.set_packages(slint::ModelRc::new(packages_to_model(&aur_pkgs)));
                                    state.set_search_result_count(search_result.resultcount);
                                    state.set_search_total_pages(((search_result.resultcount as f64) / 50.0).ceil() as i32);
                                    state.set_last_fetched_str("just now".into());
                                    state.set_status_text(
                                        format!("Found {} packages.", search_result.resultcount).into(),
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("Search failed: {}", err).into());
                                }
                            }
                        }
                        BackgroundMsg::SuggestFetched(res) => {
                            match res {
                                Ok(suggestions) => {
                                    let shared: Vec<slint::SharedString> = suggestions.into_iter().map(|s| s.into()).collect();
                                    state.set_suggestions(slint::ModelRc::new(slint::VecModel::from(shared)));
                                }
                                Err(_err) => {
                                    // Silently ignore suggest errors
                                }
                            }
                        }
                        BackgroundMsg::DetailFetched(res) => {
                            state.set_loading(false);
                            state.set_status_text("".into());

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

                                    let depends: Vec<slint::SharedString> = detail.depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_depends(slint::ModelRc::new(slint::VecModel::from(depends)));

                                    let make_depends: Vec<slint::SharedString> = detail.make_depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_make_depends(slint::ModelRc::new(slint::VecModel::from(make_depends)));

                                    let opt_depends: Vec<slint::SharedString> = detail.opt_depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_opt_depends(slint::ModelRc::new(slint::VecModel::from(opt_depends)));

                                    let check_depends: Vec<slint::SharedString> = detail.check_depends.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_check_depends(slint::ModelRc::new(slint::VecModel::from(check_depends)));

                                    let provides: Vec<slint::SharedString> = detail.provides.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_provides(slint::ModelRc::new(slint::VecModel::from(provides)));

                                    let conflicts: Vec<slint::SharedString> = detail.conflicts.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_conflicts(slint::ModelRc::new(slint::VecModel::from(conflicts)));

                                    let replaces: Vec<slint::SharedString> = detail.replaces.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_replaces(slint::ModelRc::new(slint::VecModel::from(replaces)));

                                    let groups: Vec<slint::SharedString> = detail.groups.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_groups(slint::ModelRc::new(slint::VecModel::from(groups)));

                                    let keywords: Vec<slint::SharedString> = detail.keywords.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_keywords(slint::ModelRc::new(slint::VecModel::from(keywords)));

                                    let co_maintainers: Vec<slint::SharedString> = detail.co_maintainers.into_iter().map(|s| s.into()).collect();
                                    state.set_detail_co_maintainers(slint::ModelRc::new(slint::VecModel::from(co_maintainers)));
                                    state.set_detail_submitter(detail.submitter.into());
                                    state.set_detail_first_submitted(
                                        chrono::DateTime::<chrono::Utc>::from_timestamp(detail.first_submitted, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default().into()
                                    );
                                    state.set_detail_last_modified(
                                        chrono::DateTime::<chrono::Utc>::from_timestamp(detail.last_modified, 0)
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default().into()
                                    );
                                    state.set_detail_out_of_date(
                                        detail.out_of_date
                                            .and_then(|ts| ts.parse::<i64>().ok())
                                            .and_then(|ts| chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0))
                                            .map(|d| d.format("%Y-%m-%d %H:%M UTC").to_string())
                                            .unwrap_or_default().into()
                                    );
                                }
                                Err(err) => {
                                    state.set_status_text(format!("Detail fetch failed: {}", err).into());
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
    pkgs
        .iter()
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