# Full AUR Package Browser - Implementation Plan

## Current Architecture
- **Tabs**: "Last Updated" (RSS modified), "Newest" (RSS newest)
- **Package List**: Simple cards with name, version, votes, popularity, description, maintainer, updated
- **Search**: Client-side filter only
- **Cache**: Offline-first JSON cache for "last updated" feed only

## Target Architecture: Full AUR Browser

### 1. Data Layer (client.rs) - RPC v5 Types & API

#### New Types (matching OpenAPI schemas)
```rust
// BaseResult - common fields
struct BaseResult {
    resultcount: i32,
    type: String,      // "search", "multiinfo", "error", "msearch", "suggest"
    version: i32,      // always 5
    error: Option<String>,
}

// PackageBasic - from search results
struct PackageBasic {
    id: i64,
    name: String,
    description: Option<String>,
    package_base_id: i64,
    package_base: String,
    maintainer: Option<String>,
    num_votes: i32,
    popularity: f64,
    first_submitted: i64,      // Unix timestamp
    last_modified: i64,        // Unix timestamp
    out_of_date: Option<String>, // Unix timestamp or null
    version: String,
    url_path: Option<String>,
    url: Option<String>,
}

// PackageDetailed - from info endpoint (extends PackageBasic)
struct PackageDetailed {
    // ... all PackageBasic fields +
    submitter: String,
    license: Vec<String>,
    depends: Vec<String>,
    make_depends: Vec<String>,
    opt_depends: Vec<String>,
    check_depends: Vec<String>,
    provides: Vec<String>,
    conflicts: Vec<String>,
    replaces: Vec<String>,
    groups: Vec<String>,
    keywords: Vec<String>,
    co_maintainers: Vec<String>,
}

// Search request/response
struct SearchRequest {
    arg: String,
    by: Option<SearchBy>,  // name, name-desc, depends, checkdepends, optdepends, makedepends, maintainer, submitter
}

enum SearchBy { Name, NameDesc, Depends, CheckDepends, OptDepends, MakeDepends, Maintainer, Submitter }

// Multi-info request
struct MultiInfoRequest {
    arg: Vec<String>,  // package names
}

// Suggest response
type SuggestResponse = Vec<String>;  // package names

// API Functions
fn search_packages(req: SearchRequest) -> Result<SearchResult, FetchError>
fn get_package_info(name: &str) -> Result<PackageDetailed, FetchError>
fn get_multi_info(names: &[&str]) -> Result<Vec<PackageDetailed>, FetchError>
fn suggest_packages(prefix: &str) -> Result<Vec<String>, FetchError>
fn suggest_pkgbases(prefix: &str) -> Result<Vec<String>, FetchError>
```

#### Endpoint Mapping
| OpenAPI Path | Method | Function |
|---|---|---|
| `/rpc/v5/search/{arg}` | GET | `search_packages` (path form) |
| `/rpc/v5/search` | GET/POST | `search_packages` (query/body form) |
| `/rpc/v5/info/{arg}` | GET | `get_package_info` |
| `/rpc/v5/info` | GET/POST | `get_multi_info` with `arg[]` |
| `/rpc/v5/suggest/{arg}` | GET | `suggest_packages` |
| `/rpc/v5/suggest-pkgbase/{arg}` | GET | `suggest_pkgbases` |

---

### 2. UI Layer (app.slint) - Master-Detail Layout

#### New Global State
```slint
export global AppState {
    // Existing
    in-out property <[PackageItem]> packages: [];
    in-out property <bool> loading: false;
    in-out property <string> status-text: "Ready";
    in-out property <int> active-tab: 0;  // 0=LastUpdated, 1=Newest, 2=Search, 3=Installed?
    in-out property <string> last-fetched-str: "";
    in-out property <string> search-filter: "";

    // NEW: Search tab state
    in-out property <string> search-query: "";
    in-out property <int> search-by: 0;  // index into SearchBy enum
    in-out property <int> search-page: 1;
    in-out property <int> search-total-pages: 1;
    in-out property <int> search-result-count: 0;
    in-out property <bool> search-loading: false;

    // NEW: Detail view state
    in-out property <bool> show-detail: false;
    in-out property <string> detail-name: "";
    in-out property <string> detail-version: "";
    in-out property <string> detail-description: "";
    in-out property <string> detail-maintainer: "";
    in-out property <int> detail-votes: 0;
    in-out property <float> detail-popularity: 0.0;
    in-out property <string> detail-url: "";
    in-out property <string> detail-license: "";
    in-out property <[string]> detail-depends: [];
    in-out property <[string]> detail-make-depends: [];
    in-out property <[string]> detail-opt-depends: [];
    in-out property <[string]> detail-check-depends: [];
    in-out property <[string]> detail-provides: [];
    in-out property <[string]> detail-conflicts: [];
    in-out property <[string]> detail-replaces: [];
    in-out property <[string]> detail-groups: [];
    in-out property <[string]> detail-keywords: [];
    in-out property <[string]> detail-co-maintainers: [];
    in-out property <string> detail-first-submitted: "";
    in-out property <string> detail-last-modified: "";
    in-out property <string> detail-out-of-date: "";
    in-out property <string> detail-submitter: "";
}
```

#### New Callbacks
```slint
export global AppLogic {
    callback refresh-requested();
    callback open-pkg-page(string);
    callback tab-changed(int);
    
    // NEW: Search
    callback search-requested(string query, int by, int page);
    callback search-page-changed(int page);
    callback search-by-changed(int by);
    
    // NEW: Detail view
    callback show-package-detail(string name);
    callback close-detail();
    
    // NEW: Suggestions
    callback get-suggestions(string prefix);
}
```

#### Layout: Three-Pane (Responsive)
```
┌─────────────────────────────────────────────────────────────┐
│ Header: Title | Tabs [Last Updated] [Newest] [Search] [★]  │
├──────────────┬──────────────────────────────┬───────────────┤
│              │                              │               │
│  SIDE PANEL  │      PACKAGE LIST            │  DETAIL PANE  │
│  (Filters,   │  (scrollable, selectable)    │  (collapsible,│
│   Search     │                              │   right side) │
│   History)   │                              │               │
│              │                              │               │
└──────────────┴──────────────────────────────┴───────────────┘
```

**Responsive behavior:**
- Desktop (≥1000px): Three panes visible
- Tablet (700-1000px): List + Detail (side panel as drawer)
- Mobile (<700px): Single pane with tabs/drawer for detail

---

### 3. Features by Priority

#### Phase 1: Core Search (Week 1)
- [ ] RPC v5 types & client functions
- [ ] Search tab with query input, `by` dropdown, pagination
- [ ] Debounced search (300ms)
- [ ] Search results in list (reuse PackageCard)
- [ ] Keyboard navigation (Enter to search, Esc to clear)

#### Phase 2: Package Detail View (Week 1-2)
- [ ] Detail pane component (right side)
- [ ] `show-package-detail` callback → fetch `/info/{name}`
- [ ] Display: description, dependencies (collapsible sections), licenses, keywords, metadata
- [ ] "Open in browser" button in detail
- [ ] Click package in list → show detail (not just open browser)

#### Phase 3: Autocomplete & Polish (Week 2)
- [ ] `suggest_packages` for search input autocomplete
- [ ] Search history (persisted)
- [ ] Loading skeletons for list items
- [ ] Error states with retry
- [ ] Keyboard shortcuts (/, Escape, arrows)

#### Phase 4: Advanced Features (Week 2-3)
- [ ] Multi-info batch fetch for visible packages (enrich list with votes/popularity)
- [ ] Installed packages tab (local pacman query)
- [ ] Package base grouping
- [ ] Export/search sharing (URL with query params)
- [ ] Theme persistence

---

### 4. Technical Decisions

#### Caching Strategy
- **RSS feeds**: Keep existing 6-hour cache
- **Search results**: No cache (dynamic)
- **Package details**: LRU cache (50 entries, 10 min TTL) in memory
- **Suggestions**: In-memory, 5 min TTL

#### Pagination
- AUR returns max ~100 results per page
- Implement "Load More" button + infinite scroll
- Track `page` and `resultcount` for total pages

#### Error Handling
- Network errors → toast + retry button
- Rate limiting (AUR: 1 req/sec per IP) → queue + backoff
- Malformed responses → log + show generic error

#### Threading
- Keep blocking `reqwest` on background threads
- Use `mpsc` channels for UI updates
- One channel per request type or unified `BackgroundMsg` enum

---

### 5. File Changes Summary

| File | Changes |
|---|---|
| `src/client.rs` | Add RPC v5 types, `search_packages`, `get_package_info`, `get_multi_info`, `suggest_packages`, `suggest_pkgbases` |
| `src/main.rs` | Add search/detail callbacks, new `BackgroundMsg` variants, detail state management, pagination logic |
| `ui/app.slint` | New `PackageDetail` struct, `AppState` extensions, `AppLogic` callbacks, three-pane layout, detail component, search tab UI |
| `Cargo.toml` | Add `url` crate for query building, maybe `lru` for detail cache |
| `src/storage.rs` | Extend for search history, detail cache (optional) |

---

### 6. OpenAPI-Derived Validation

All enum values, field names, and endpoint paths verified against `https://aur.archlinux.org/rpc/openapi.json`:
- `SearchBy` enum: 8 values exactly as spec
- `PackageBasic` / `PackageDetailed` fields match `allOf` schema composition
- `InfoBody` uses `arg[]` array with `form` encoding
- No `msearch` or `packages` endpoints in spec → skip

---

### 7. Acceptance Criteria

1. **Search tab**: Query "yay" → shows 50+ results with pagination
2. **Detail view**: Click any package → right pane shows full metadata
3. **Autocomplete**: Type "ya" → dropdown shows "yay", "yay-bin", etc.
4. **Responsive**: Works at 700px, 1000px, 1400px widths
5. **Offline**: Cached "Last Updated" loads instantly on startup
6. **Performance**: 60fps scrolling, <200ms search latency (local debounce)
7. **Tests**: Unit tests for all new client functions, integration test for search flow