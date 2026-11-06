slint::include_modules!();

mod client;
mod storage;

use client::{AurPackage, RssPackage};
use slint::{VecModel};
use std::sync::mpsc;
use std::thread;

enum BackgroundMsg {
    LastUpdatedFetched(Result<Vec<AurPackage>, String>),
    RssFetched(Result<Vec<RssPackage>, String>),
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
            } else {
                trigger_rss_fetch(&app, &tx_refresh);
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
            } else {
                trigger_rss_fetch(&app, &tx_tab);
            }
        }
    });

    ui.global::<AppLogic>().on_open_pkg_page(move |pkg_name| {
        let url = format!("https://aur.archlinux.org/packages/{pkg_name}");
        let _ = open::that(&url);
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