# AURbreak

<p align="center">
  <img src="ui/assets/aurbreak-icon-app.svg" alt="AURbreak Logo" width="128" height="128" />
</p>

<p align="center">
  <strong>A very fast lightweight offline-first app for for browsing, searching, and inspecting packages from the Arch User Repository (AUR).</strong>
</p>

<p align="center">
  <a href="https://github.com/snowballons/AURbreak/actions/workflows/ci.yml"><img src="https://github.com/snowballons/AURbreak/actions/workflows/ci.yml/badge.svg" alt="CI Status" /></a>
  <a href="https://aur.archlinux.org/packages/aurbreak-bin"><img src="https://img.shields.io/aur/version/aurbreak-bin?color=1793d1&label=AUR" alt="AUR Package" /></a>
  <a href="https://github.com/snowballons/AURbreak/releases"><img src="https://img.shields.io/github/v/release/snowballons/AURbreak?color=orange" alt="Latest Release" /></a>
  <a href="https://www.gnu.org/licenses/gpl-3.0"><img src="https://img.shields.io/badge/License-GPL%203.0-blue.svg" alt="GPL-3.0 License" /></a>
  <a href="https://slint.dev"><img src="https://img.shields.io/badge/GUI-Slint-27a7d8.svg" alt="Slint GUI" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Language-Rust-dea584.svg" alt="Rust" /></a>
</p>

---

## Overview

**AURbreak** is a modern Linux desktop application built with [Slint](https://slint.dev) and [Rust](https://www.rust-lang.org/) for browsing, searching, and inspecting packages from the [Arch User Repository (AUR)](https://aur.archlinux.org). 

It connects directly to the official AUR RPC v5 endpoints and RSS feeds, pairs high performance with low memory usage, caches data locally for instant offline review, and provides one-click installation commands for your favorite AUR helper (`yay`, `paru`, `pikaur`).

```
┌────────────────────────────────────────────────────────────────────────┐
│ AURbreak (Slint UI)                                                    │
│                                                                        │
│  [Logo] AURbreak      [ ⚡ Last Modified | ✨ Newest | 🔍 Search AUR ]  [⚙] [↻] │
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
│  ┌────────────────────────────────┐  │  Upstream URL: https://...      │
│  │ yay v12.4.2-1                  │  │  License: GPL-3.0-or-later      │
│  │ Pacman wrapper and AUR helper  │  │  ─────────────────────────────  │
│  │ ▲ 4120 votes  ★ 68.30 pop      │  │  ▲ 2450 votes  ★ 42.15 pop      │
│  └────────────────────────────────┘  │  Maintainer: maintainer_name    │
│                                      │  Dependencies (3):              │
│                                      │  [ • zlib ] [ • glibc ]         │
├──────────────────────────────────────┴─────────────────────────────────┤
│ Ready • 100 packages loaded               Esc: Close details • Enter: Search │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Key Features

- **⚡ Live Feed Discovery**:
  - **Last Modified**: Browse the freshest updates via the official AUR RSS feed.
  - **Newest Submissions**: Discover newly registered AUR packages.
- **🔍 Full AUR RPC v5 Search**:
  - Filter by **Name**, **Name & Description**, **Maintainer**, **Submitter**, or **Dependencies** (`depends`, `makedepends`, `optdepends`, `checkdepends`).
  - Fast instant client-side sub-filtering on loaded results.
  - Paginated browsing with customizable page sizes (25, 50, 100, 250).
- **📋 Deep Package Inspector**:
  - Full package details: Votes, Popularity score, Maintainer, Submitter, First Submitted, and Last Modified timestamps.
  - Upstream URL with direct browser launcher.
  - Granular dependency tree tags: Runtime Depends, Make Depends, and Optional Depends.
  - Flagged out-of-date alert banner.
  - Direct `AUR ↗` button opening the official package page.
- **💻 Quick Terminal Helper Integration**:
  - One-click copyable terminal command for your preferred AUR helper (`yay -S <pkg>`, `paru -S <pkg>`, or `pikaur -S <pkg>`).
- **💾 Offline-First Architecture**:
  - Instant startup with persistent local caching under XDG data directory (`~/.local/share/aurbreak/cache.json`).
  - Automatic cache backup (`cache.json.bak`) with corruption self-healing.
- **⚙️ Configurable Preferences**:
  - Customize preferred AUR helper, default search criteria, search result limits, and offline cache management.
- **🎨 Adaptive Theme**:
  - Native light and dark theme switching designed for Linux desktops.

---

## Installation

### 1. Arch Linux (AUR)

Install via your AUR helper of choice:

```bash
# Using yay
yay -S aurbreak-bin

# Using paru
paru -S aurbreak-bin

# Using pikaur
pikaur -S aurbreak-bin
```

Or build manually with `makepkg`:

```bash
git clone https://aur.archlinux.org/aurbreak-bin.git
cd aurbreak-bin
makepkg -si
```

### 2. Arch Linux Pre-built Package (`.pkg.tar.zst`)

Download the `.pkg.tar.zst` package from the [Releases](https://github.com/snowballons/AURbreak/releases) page:

```bash
sudo pacman -U aurbreak-<version>-1-x86_64.pkg.tar.zst
```

### 3. Universal AppImage

The AppImage runs on any modern Linux distribution without installation:

```bash
chmod +x AURbreak-<version>-x86_64.AppImage
./AURbreak-<version>-x86_64.AppImage
```

### 4. Debian / Ubuntu (`.deb`)

Download the `.deb` package from [Releases](https://github.com/snowballons/AURbreak/releases):

```bash
sudo dpkg -i aurbreak_<version>_amd64.deb
# If dependencies are missing:
sudo apt-get install -f
```

### 5. Fedora / RHEL / openSUSE (`.rpm`)

Download the `.rpm` package from [Releases](https://github.com/snowballons/AURbreak/releases):

```bash
# Fedora / RHEL
sudo dnf install ./aurbreak-<version>-1.x86_64.rpm

# openSUSE
sudo zypper install ./aurbreak-<version>-1.x86_64.rpm
```

### 6. Generic Portable Tarball

Download and extract the portable `.tar.gz`:

```bash
tar -xzf aurbreak-<version>-x86_64-unknown-linux-gnu.tar.gz
cd aurbreak-<version>-x86_64-unknown-linux-gnu

# Install system-wide (/usr/local):
sudo ./install.sh

# Or install for current user only (~/.local):
./install.sh --user
```

---

## Building from Source

### Prerequisites

Ensure you have Rust (≥ 1.75) and system libraries installed.

**Arch Linux**:
```bash
sudo pacman -S --needed rust cargo git fontconfig libxkbcommon pkg-config
```

**Debian / Ubuntu**:
```bash
sudo apt-get install -y build-essential curl pkg-config libfontconfig1-dev libxkbcommon-dev libssl-dev
```

**Fedora**:
```bash
sudo dnf install -y gcc gcc-c++ rust cargo fontconfig-devel libxkbcommon-devel openssl-devel
```

### Build & Run

```bash
# Clone the repository
git clone https://github.com/snowballons/AURbreak.git
cd AURbreak

# Run unit tests
cargo test

# Build optimized release binary
cargo build --release

# Launch application
./target/release/aurbreak
```

---

## Keyboard Shortcuts & Navigation

| Key / Action | Context | Description |
|---|---|---|
| <kbd>Enter</kbd> | Search Bar | Submit query to AUR RPC v5 |
| <kbd>Esc</kbd> | Package Inspector | Close the package detail inspector pane |
| <kbd>Tab</kbd> / <kbd>Shift+Tab</kbd> | Global | Navigate focus between controls |
| Click outside pane | Global | Dismiss settings or inspector views |

---

## Storage & Configuration Paths

AURbreak strictly follows the [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/basedir-spec-latest.html):

| File | Path | Description |
|---|---|---|
| **User Settings** | `~/.local/share/aurbreak/config.json` | Stores preferred AUR helper, search category, and limits. |
| **Offline Cache** | `~/.local/share/aurbreak/cache.json` | Persists package feed data for instant startup and offline usage. |
| **Cache Backup** | `~/.local/share/aurbreak/cache.json.bak` | Automatic backup file used for corruption recovery. |

---

## Technology Stack

- **GUI Framework**: [Slint](https://slint.dev/) (Native, fluid declarative UI with software & GPU rasterizers).
- **Core Language**: [Rust](https://www.rust-lang.org/) (Memory-safe, high performance, zero runtime overhead).
- **HTTP Client**: [reqwest](https://docs.rs/reqwest) with `rustls-tls` (pure-Rust TLS stack).
- **Serialization**: [serde](https://serde.rs/) & `serde_json` for robust OpenAPI v5 deserialization.
- **Date & Time**: [chrono](https://docs.rs/chrono) for RFC 2822 RSS feeds and ISO 8601 timestamps.
- **System Integration**: [open](https://docs.rs/open) for launching external web links in default browsers.

---

## Continuous Integration & Release Automation

The repository includes complete GitHub Actions workflows:

- **`.github/workflows/ci.yml`**:
  - Automated formatting checks (`cargo fmt --check`).
  - Lints & compiler warnings (`cargo clippy -- -D warnings`).
  - Comprehensive unit test suite execution.
  - Headless UI validation & snapshot generation via `xvfb-run`.
- **`.github/workflows/release.yml`**:
  - Multi-distribution build matrix: `.pkg.tar.zst` (Arch), `.deb` (Debian/Ubuntu), `.rpm` (Fedora), universal `.AppImage`, and `.tar.gz`.
  - Automatic `SHA256SUMS.txt` generation and GitHub Release publication.
  - Automated deployment to the **Arch User Repository (AUR)** via SSH keys on release tags (`v*`).

---

## Contributing

Contributions, bug reports, and suggestions are welcome!

1. Fork the repository.
2. Create your feature branch (`git checkout -b feature/amazing-feature`).
3. Ensure formatting and tests pass:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```
4. Commit your changes (`git commit -m 'feat: add amazing feature'`).
5. Open a Pull Request.

---

## License

This project is licensed under the **GNU General Public License v3.0 or later** ([GPL-3.0-or-later](LICENSE)).
