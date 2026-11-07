# AURbreak — Project Roadmap & Architecture

A fast, offline-first desktop browser for the Arch User Repository (AUR), built with [Slint](https://slint.dev) and Rust.

---

## 1. Project Overview & Architecture

AURbreak provides a modern, responsive GUI for exploring, searching, and inspecting AUR packages. It connects to the official AUR RPC v5 API and RSS feeds, provides offline caching, and features a clean master-detail inspection experience.

```
┌────────────────────────────────────────────────────────────────────────┐
│ AURbreak (Slint UI)                                                    │
│                                                                        │
│  [Logo] AURbreak      [ ⚡ Last Modified | ✨ Newest | 🔍 Search AUR ]  [↻ Refresh]
├────────────────────────────────────────────────────────────────────────┤
│  [ 🔍 Filter packages by name, description, maintainer... ] [100 pkgs] │
├──────────────────────────────────────┬─────────────────────────────────┤
│  PACKAGE LIST                        │  INSPECTOR PANE (Width: 420px)  │
│  ──────────────────────────────────  │  ─────────────────────────────  │
│  ┌────────────────────────────────┐  │  pkg-name v1.2.3    [AUR ↗] [✕] │
│  │ visual-studio-code-bin v1.96-1 │  │  ─────────────────────────────  │
│  │ Description text...            │  │  >_ yay -S pkg-name             │
│  │ ▲ 2450 votes  ★ 42.15 pop      │  │  ─────────────────────────────  │
│  └────────────────────────────────┘  │  ABOUT                          │
│  ┌────────────────────────────────┐  │  Full description text...       │
│  │ yay v12.4.2-1                  │  │  ─────────────────────────────  │
│  │ Pacman wrapper and AUR helper  │  │  ▲ 2450 votes  ★ 42.15 pop      │
│  │ ▲ 4120 votes  ★ 68.30 pop      │  │  Maintainer: ...                │
│  └────────────────────────────────┘  │  Dependencies (3):              │
│                                      │  [ • zlib ] [ • glibc ]         │
├──────────────────────────────────────┴─────────────────────────────────┤
│ Ready • 100 packages loaded               Esc: Close details • Enter: Search │
└────────────────────────────────────────────────────────────────────────┘
```

### Architecture Highlights

- **UI Framework**: Slint ≥ 1.17 with custom `Theme` semantic design tokens for automatic Light/Dark mode switching based on OS theme.
- **Backend & Network**: Rust with `reqwest` (rustls-tls) running exclusively on background worker threads, communicating with the UI thread via non-blocking `mpsc` channels and a 100ms timer poll loop.
- **Offline Cache**: Single-file atomic JSON cache under XDG data dir (`~/.local/share/browse-arch/cache.json`) with `.bak` corruption recovery.
- **Data Source**: Official AUR RPC v5 endpoints (`https://aur.archlinux.org/rpc/v5/`) + official RSS modified/newest feeds.

---

## 2. Phase Status Matrix

| Phase | Description | Status |
|---|---|:---:|
| **Phase 1** | Core Search & RPC v5 Integration | ✅ Complete |
| **Phase 2** | Package Detail View & Inspector | ✅ Complete |
| **Phase 2.5** | UI/UX Overhaul & Layout Restructuring | ✅ Complete |
| **Phase 3** | Autocomplete & Search Polish | 🟡 In Progress |
| **Phase 3.5** | Settings & Preferences Configuration | ✅ Complete |
| **Phase 4** | Advanced Features (Installed, Multi-info, Grouping) | ⬜ Planned |
| **Phase 5** | Testing, CI & Linux Packaging | ✅ Complete |

---

## 3. Detailed Phase Breakdown

### Phase 1: Core Search & RPC v5 Integration ✅ (Complete)

- [x] **RPC v5 Types & Schema Deserialization (`client.rs`)**:
  - `PackageBasic` and `PackageDetailed` mapped to AUR OpenAPI v5 specs.
  - Implemented `#[serde(default)]` on all optional fields to safely deserialize packages with missing metadata.
  - `SearchBy` enum supporting all 8 official search criteria (`name`, `name-desc`, `depends`, `checkdepends`, `optdepends`, `makedepends`, `maintainer`, `submitter`).
- [x] **RPC Client Functions (`client.rs`)**:
  - `search_packages(query, by)` via `/rpc/v5/search`.
  - `get_package_info(name)` via `/rpc/v5/info/{name}`.
  - `get_multi_info(names)` via `/rpc/v5/info?arg[]=...`.
  - `suggest_packages(prefix)` via `/rpc/v5/suggest/{prefix}`.
  - `suggest_pkgbases(prefix)` via `/rpc/v5/suggest-pkgbase/{prefix}`.
- [x] **RSS Feed Integration**:
  - `fetch_last_updated()` parses the official `/rss/modified` feed.
  - `fetch_rss_newest()` parses `/rss/` (handling 307 redirects).
  - RFC 2822 timestamp parsing into UTC `DateTime`.
- [x] **Search Tab UI**:
  - Integrated `SearchBar` with search query input, `ComboBox` category selector, and primary search button.
  - Pagination controls ([◀ Prev], Page X of Y, [Next ▶]) and total result counts.

---

### Phase 2: Package Detail View & Inspector ✅ (Complete)

- [x] **Detail Inspector Pane (`ui/app.slint`)**:
  - Side-by-side inspector layout (`width: 420px;`) rendered conditionally (`if AppState.show-detail:`).
  - Selected card visual highlight with accent border and active card background.
- [x] **Metadata Display**:
  - Header with package name, version pill badge, `AUR ↗` web button, and `✕` close button.
  - Quick install terminal snippet: `>_ yay -S <pkg>` with helper notice.
  - Flagged out-of-date alert banner.
  - Full description block.
  - 2-column key stats card: Votes, Popularity, Maintainer, Submitter, First Submitted, Last Modified.
  - License tag.
  - Grouped dependency cards (`depends`, `make_depends`, `opt_depends`, `provides`, `conflicts`).
- [x] **External Link Launching**:
  - Integrated `open::that` to open `https://aur.archlinux.org/packages/{name}` in the user's default browser.

---

### Phase 2.5: UI/UX Overhaul & Layout Redesign ✅ (Complete)

- [x] **Layout & Proportions Fixes**:
  - Fixed vertical stretch bug where header buttons stretched into 200px tall monoliths.
  - Replaced oversized button widgets with a modern, compact segmented pill control (`⚡ Last Modified`, `✨ Newest`, `🔍 Search AUR`).
  - Set root `VerticalLayout` to `width: 100%; height: 100%` so the list fills the entire window, eliminating the empty black void at the bottom.
  - Constrained card heights with `vertical-stretch: 0` and `alignment: start` so cards size naturally (~85px) rather than stretching.
- [x] **Real-time Client-Side Filtering**:
  - Replaced broken Slint exact equality filter (`pkg.name == filter`) with high-speed Rust-side case-insensitive substring matching across `name`, `description`, and `maintainer`.
  - Added dynamic package count badge ("Showing X of Y packages") and quick-clear button (`✕ Clear`).
- [x] **Search UI Modernization**:
  - Replaced awkward click-to-cycle rectangle with native Slint `ComboBox`.
  - Bound <kbd>Enter</kbd> key inside search input to immediately trigger search.
- [x] **Keyboard Navigation**:
  - Added full-window `FocusScope` intercepting <kbd>Escape</kbd> to close the detail inspector.
- [x] **High-Fidelity Theming**:
  - Polished color palettes for both Dark Mode (`#0b0f19` canvas, `#131d2e` cards) and Light Mode (`#f8fafc` canvas, `#ffffff` cards).
- [x] **Snapshot Verification Tool**:
  - Added `--snapshot <out_dir>` CLI mode using `ui.window().take_snapshot()` and ImageMagick for headless regression testing.

---

### Phase 3: Autocomplete & Search Experience Polish 🟡 (In Progress)

- [x] Basic suggestions API (`suggest_packages` client endpoint & `get-suggestions` callback wired).
- [ ] **300ms Debounce Timer**:
  - Add timer for autocomplete suggestion queries to avoid spamming the AUR RPC while typing.
- [ ] **Search History**:
  - Persist the last 10 search queries locally in `storage.rs`.
  - Render recent search queries as clickable chips beneath the search bar.
  - Provide a "Clear History" button.
- [ ] **Loading Skeletons**:
  - Display skeleton placeholder cards in the list while `search-loading` is active.
- [ ] **Enhanced Error States**:
  - In-app retry button for network timeouts and HTTP errors.
  - Friendly rate-limit notice ("Too many requests, retrying in Ns").
- [ ] **Keyboard Suggestion Navigation**:
  - Navigate suggestion dropdown items via <kbd>↑</kbd> and <kbd>↓</kbd> arrow keys.
  - Autofill search input with selected suggestion on <kbd>Tab</kbd> or <kbd>Enter</kbd>.

---

### Phase 3.5: Settings & User Preferences ✅ (Complete)

- [x] **Settings Button & Navigation**:
  - Dedicated `[ ⚙ Settings ]` header button next to Refresh with active state indicator.
  - Full-page responsive `SettingsView` with `[ ← Back to Packages ]` and <kbd>Esc</kbd> shortcut handler.
- [x] **Configuration Persistence (`storage.rs`)**:
  - Atomic JSON configuration persistence (`~/.local/share/browse-arch/config.json`).
  - Unit tests for configuration roundtrip and recovery.
- [x] **Appearance & Theme Switching**:
  - Real-time live switching between `System Default`, `Dark Theme`, and `Light Theme`.
  - Persisted user preference automatically applied at startup.
- [x] **AUR Helper Integration**:
  - Configurable preferred AUR helper (`yay`, `paru`, `pikaur`, `aura`, `pacman`).
  - Automatically updates install commands (`>_ <helper> -S <pkg>`) in package inspector.
- [x] **Search & Pagination Defaults**:
  - Configurable default search criterion (`SearchBy` index 0..7).
  - Configurable search page size (`25`, `50`, `100` results).
- [x] **Offline Cache Management**:
  - Cache file path and formatted disk usage display.
  - Interactive `[ 🗑 Clear Cache ]` button with atomic deletion and status feedback.
- [x] **Upstream Service & About**:
  - Official AUR RPC v5 and RSS endpoint references with OpenAPI link.
  - App branding, version tag (`v0.1.0`), and Arch Linux portal links.

---

### Phase 4: Advanced Features ⬜ (Planned)

- [ ] **Multi-Info Batch Fetch**:
  - Utilize existing `get_multi_info(&[&str])` to fetch enriched metadata (exact vote counts, popularity, maintainer) for visible packages in RSS feeds.
- [ ] **Installed Packages Tab**:
  - Query local `pacman -Qm` to list foreign/AUR-installed packages on the system.
  - Cross-reference local package versions with AUR RPC v5 info to display "Update Available" badges.
  - Add Tab 3: "Installed".
- [ ] **PackageBase Grouping**:
  - Group search results sharing the same `PackageBase` (e.g., `yay`, `yay-bin`, `yay-git`).
  - Expandable group headers with base package details.
- [ ] **Search Sharing & URL Handling**:
  - Support startup arguments and URL scheme parsing (e.g. `browse-arch --search yay --by name-desc`).
- [ ] **Context Menu**:
  - Right-click context menu on `PackageCard` using Slint's `ContextMenuArea`: "Copy package name", "Copy install command", "Open in AUR".

---

### Phase 5: Testing, CI & Linux Packaging ✅ (Complete)

- [x] **Unit Testing**:
  - 18 unit tests covering deserialization, missing optional fields handling, RFC 2822 date parsing, search by serialization, config persistence, and offline cache corruption recovery.
- [x] **Headless Snapshot Testing**:
  - Automated generation of light and dark mode snapshots across all views via `--snapshot`.
- [x] **Cargo Profile Optimizations**:
  - Thin LTO and symbols stripping in `[profile.release]`.
  - Dependency caching optimization in `[profile.dev]`.
- [x] **Continuous Integration (`.github/workflows/ci.yml`)**:
  - Runs on pull requests and pushes to `main`/`master`.
  - `cargo fmt` check, `cargo clippy` with `-D warnings`, `cargo test`, and headless UI snapshot verification uploading snapshot artifacts.
- [x] **Multi-Distro Release Automation (`.github/workflows/release.yml`)**:
  - **Arch Linux**: Official `.pkg.tar.zst` built via `archlinux:base-devel` container.
  - **Debian / Ubuntu**: `.deb` package via `cargo-deb` with desktop integration.
  - **Fedora / RHEL / openSUSE**: `.rpm` package via `cargo-generate-rpm`.
  - **Universal AppImage**: Portable standalone `.AppImage` via `appimagetool`.
  - **Generic Tarball**: Portable `.tar.gz` with `install.sh` installer script.
  - **Automated Publishing**: Generates `SHA256SUMS.txt` and publishes GitHub Release assets on version tags (`v*`).

---

## 4. Technical Specifications & File Layout

| File | Purpose |
|---|---|
| [`src/main.rs`](file:///home/jonnie/work/tries/browse-arch/browse-arch/src/main.rs) | Application lifecycle, background thread communication, filter logic, event loop poll, snapshot utility. |
| [`src/client.rs`](file:///home/jonnie/work/tries/browse-arch/browse-arch/src/client.rs) | AUR RPC v5 client, OpenAPI schemas, RSS parsing, error handling, client unit tests. |
| [`src/storage.rs`](file:///home/jonnie/work/tries/browse-arch/browse-arch/src/storage.rs) | XDG atomic cache persistence, user settings config, load/save logic, corrupt file recovery tests. |
| [`ui/app.slint`](file:///home/jonnie/work/tries/browse-arch/browse-arch/ui/app.slint) | Slint GUI: `Theme`, `AppState`, `AppLogic`, `NavTab`, `PackageCard`, `SearchBar`, `PackageDetail`, `SettingsView`, `AppWindow`. |
| [`.github/workflows/ci.yml`](file:///home/jonnie/work/tries/browse-arch/browse-arch/.github/workflows/ci.yml) | GitHub Actions CI: formatting, clippy lints, unit testing, and headless UI snapshot verification. |
| [`.github/workflows/release.yml`](file:///home/jonnie/work/tries/browse-arch/browse-arch/.github/workflows/release.yml) | GitHub Actions Release: packaging for Arch Linux (.pkg.tar.zst), Debian/Ubuntu (.deb), Fedora (.rpm), AppImage, and tarball. |
| [`packaging/aurbreak.desktop`](file:///home/jonnie/work/tries/browse-arch/browse-arch/packaging/aurbreak.desktop) | XDG desktop entry specification for Linux desktop application menus. |
| [`packaging/PKGBUILD`](file:///home/jonnie/work/tries/browse-arch/browse-arch/packaging/PKGBUILD) | Arch Linux packaging recipe for AUR distribution. |
| [`packaging/install.sh`](file:///home/jonnie/work/tries/browse-arch/browse-arch/packaging/install.sh) | Portable shell installer for system-wide or user-local installation. |
| [`Cargo.toml`](file:///home/jonnie/work/tries/browse-arch/browse-arch/Cargo.toml) | Dependencies (`slint`, `reqwest`, `serde`, `chrono`, `open`), deb/rpm metadata, build profiles. |
| [`ROADMAP.md`](file:///home/jonnie/work/tries/browse-arch/browse-arch/ROADMAP.md) | High-level roadmap, architecture documentation, and phase progress tracker. |
