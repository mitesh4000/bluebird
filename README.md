# Bluebird

Pure-Rust native desktop HTTP API client. No Electron, no webview, no JavaScript at runtime.

**Stack:** Rust 1.80+ · egui 0.29 / eframe 0.29 · reqwest (rustls) · rusqlite · arboard · keyring · rfd

## Features

- Request composer (method, URL, headers, body types, auth, SSL verify, timeout)
- Send / Cancel with background thread + abort flag
- Paste `curl` into the URL field → auto-import
- Copy as cURL (optional secret redaction)
- JSON response tree with click-to-copy path (JS / JSONPath / variable)
- History & saved requests (SQLite)
- Blue-and-black theme, keyboard shortcuts, searchable help

## Build & run

```bash
# Linux dependencies (Debian/Ubuntu example)
sudo apt-get install -y build-essential libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

cargo run --release
```

**macOS:** Xcode CLT required.  
**Windows:** MSVC toolchain + Windows SDK.

Minimum stable Rust: **1.80**.

## Tests

```bash
cargo test
```

Covers curl parser fixtures and JSON path generation (including the mandatory F5 examples).

## Shortcuts

| Action | Win/Linux | macOS |
|--------|-----------|-------|
| Send | Ctrl+Enter | Cmd+Enter |
| Cancel | Esc | Esc |
| Copy as cURL | Ctrl+Shift+C | Cmd+Shift+C |
| New tab | Ctrl+T | Cmd+T |
| Close tab | Ctrl+W | Cmd+W |
| Focus URL | Ctrl+L | Cmd+L |
| Toggle Pretty/Raw | Ctrl+Shift+F | Cmd+Shift+F |
| Shortcuts help | Ctrl+/ | Cmd+/ |
| Toggle sidebar | Ctrl+B | Cmd+B |

## Module layout

See `SPEC.md` for the full product specification, palette, architecture, security notes, phases, and acceptance criteria.

```
src/
  main.rs app.rs
  ui/          theme, request_pane, response_pane, json_tree, toast, shortcuts_help
  http/        models, client
  curl/        parser
  jsonpath/    path
  storage/     db
  security/    secrets
```

## License

MIT
# bluebird
