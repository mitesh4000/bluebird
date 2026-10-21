# Bluebird — Product Specification (v1)

## 1. Product Overview

**Bluebird** is a lightweight, offline-first native desktop HTTP API client written entirely in Rust. Users compose requests, paste curl commands into the URL field for automatic import, send requests, and inspect JSON responses via a collapsible tree with click-to-copy path generation.

**Target users:** Developers and QA engineers who test REST APIs daily and prefer a fast native tool without Electron or a webview.

**Goals:** Speed, offline operation, keyboard-first workflows, accessible blue/black UI, safe credential handling.

### Explicit assumptions

| Assumption | Rationale |
|---|---|
| Single-user, local-only, fully offline | No accounts or cloud in v1 |
| Platforms: Windows, macOS, Linux (x86_64; ARM64 where feasible) | Standard desktop targets for egui/eframe |
| JSON is the only format with click-to-copy path; other bodies are plain text | Spec scope |
| "Copy request" produces a curl command; optional JSON request copy is a setting (default off) | Spec §1 |
| No telemetry; logging is local and opt-in via `RUST_LOG` | Privacy |
| UI framework: **egui 0.29 via eframe 0.29** | See §2 |
| Minimum Rust: **1.80 stable** | Dependency MSRV |
| History retention default: 30 days | Reasonable local default |
| Multipart/binary file pick uses `rfd` native dialogs | Pure-Rust file picker |
| Cookie tab shows `Set-Cookie` response headers only (no persistent jar) | v1 scope |
| Secrets stored in OS keychain via `keyring`; plaintext only transiently in memory during send | Security §6 |

### Non-goals (v1 / documented for v2+)

Cloud sync, accounts, teams, GraphQL, WebSocket, gRPC, scripting engine, collection runner CLI, environment variables, proxy UI, automatic updates.

---

## 2. UI Framework Decision

| Criterion | egui (eframe) | iced | Slint |
|---|---|---|---|
| Maturity | High; widely used in tools | Medium–high | High (commercial + OSS) |
| Ecosystem | Large (egui ecosystem crates) | Growing | Good tooling |
| Accessibility | Keyboard focus, screen-reader limited | Improving | Stronger a11y goals |
| Complex tree UI | Custom widgets; virtualization feasible | Custom | Built-in list/tree patterns |
| Styling | Full programmatic control | Theme system | Declarative styles |
| Licensing | MIT/Apache-2.0 | MIT | GPLv3 or paid |

**Choice: egui 0.29 + eframe 0.29 (MIT/Apache-2.0).**

**Justification:** egui is pure Rust, immediate-mode, proven for developer tools, integrates with arboard/rfd, and supports custom virtualized trees with hover tooltips and keyboard focus. No webview. Licensing is permissive. Virtualization is implemented via windowed scroll of expanded nodes rather than a third-party list crate.

---

## 3. Features & User Flows

### F1 — Request composer
Method dropdown, URL input (validated), headers (enable toggles + autocomplete), body (Raw/Form/Multipart/Binary/None), auth (None/Basic/Bearer/API Key), SSL verify toggle, timeout (default 30s).

### F2 — Send / inspect
Send + Cancel (tokio abort). Response: status, time, size, headers, body. States: empty, loading, success 2xx, non-2xx, network error, cancelled, invalid URL, invalid JSON.

### F3 — curl paste import
Debounced paste detection; parse supported flags; banners for success/partial/failure; confirm replace if fields dirty.

### F4 — Copy as cURL
Button + shortcut; optional redact; first-time secret warning; toast.

### F5 — JSON click-to-copy path
Collapsible tree; three path modes; mandatory examples must pass (see acceptance).

### F6 — Persistence
SQLite history + saved requests + settings; clear history.

### F7 — Theme
Blue/black tokens (see §6 palette); WCAG AA; focus rings; icon+text for status.

### F8 — Shortcuts
See keyboard table in SPEC / README.

### UI states (wording)

| State | Wording |
|---|---|
| Empty | "Send a request to see the response." |
| Loading | "Sending…" |
| Success 2xx | Status code + reason + time + size |
| Non-2xx | Same as success layout; status colored warning/error |
| Network error | "Network error: {message}" |
| Cancelled | "Request cancelled." |
| Invalid URL | "Invalid URL." |
| Invalid JSON | "Response is not valid JSON. Path copy disabled." |
| Curl import success | "Imported from curl command." |
| Curl import partial | "Imported with warnings: {flags}" |
| Curl import failure | "Could not parse curl command." |
| Replace prompt | "Replace current request with imported curl?" |

---

## 4. Architecture

```
src/
  main.rs
  app.rs
  ui/
    mod.rs
    request_pane.rs
    response_pane.rs
    json_tree.rs
    toast.rs
    shortcuts_help.rs
    theme.rs
  http/
    mod.rs
    client.rs
    models.rs
  curl/
    mod.rs
    parser.rs
  storage/
    mod.rs
    db.rs
  security/
    mod.rs
    secrets.rs
  jsonpath/
    mod.rs
    path.rs
```

**Networking:** Shared `reqwest::Client` (rustls); per-request timeout; abort via oneshot/cancel flag; max 10 redirects (configurable); bodies >5MB offer save-to-file via rfd and show truncated preview.

---

## 5. Keyboard shortcuts

| Action | Win/Linux | macOS |
|---|---|---|
| Send | Ctrl+Enter | Cmd+Enter |
| Cancel | Esc | Esc |
| Copy as cURL | Ctrl+Shift+C | Cmd+Shift+C |
| New tab | Ctrl+T | Cmd+T |
| Close tab | Ctrl+W | Cmd+W |
| Focus URL | Ctrl+L | Cmd+L |
| Toggle Pretty/Raw | Ctrl+Shift+F | Cmd+Shift+F |
| Copy focused path | Enter | Enter |
| Shortcuts help | Ctrl+/ | Cmd+/ |
| Toggle sidebar | Ctrl+B | Cmd+B |

---

## 6. Palette (blue-and-black)

```
--bg-base:        #0B0F14
--bg-surface:     #121821
--bg-elevated:    #1A2230
--border:         #243040
--text-primary:   #E6EDF3
--text-secondary: #9DAAB8
--accent:         #2F81F7
--accent-hover:   #4C93F8
--accent-pressed: #1F6FE0
--success:        #2EA043
--warning:        #D29922
--error:          #F85149
--highlight-hover:#2F81F724  (rgba 14%)
```

Body text contrast ≈ 12:1 on bg-base. Focus: 2px accent ring, 2px offset. Status always pairs icon glyph with text.

---

## 7. Security & privacy

- Offline-only; no telemetry.
- Secrets via OS keychain (`keyring`); SQLite stores references only.
- History may hold sensitive data — clear-history provided; at-rest encryption deferred.
- First clipboard copy warns about secrets; redact-on-copy setting.
- TLS on by default; insecure toggle shows warning badge.
- Curl never reads `@file` paths.
- Logs redact secrets; `RUST_LOG` opt-in.

---

## 8. Phases & acceptance

**P1** Shell + theme + send GET/POST + response states  
**P2** curl parser + banners  
**P3** JSON tree + path modes + toast  
**P4** Copy as cURL + shortcuts + help  
**P5** SQLite + polish + packaging  

### Acceptance (pass/fail)

- F1: All method/body/auth modes populate and send.
- F2: All response states reachable with correct wording.
- F3: Fixture corpus ≥95% valid fully parsed; 100% invalid error without field mutation; unsupported flags in warnings.
- F4: Round-trip copy → re-import equivalent RequestSpec.
- F5: Mandatory path examples pass; non-JSON disables path copy.
- F6: History survives restart; clear works.
- F7: Tokens applied; focus visible; contrast AA.
- F8: All listed shortcuts work; help dialog searchable.

---

## v2+ (not implemented)

Environments, collection runner, GraphQL, WebSocket, proxy UI, cloud sync, at-rest DB encryption, multi-window.
