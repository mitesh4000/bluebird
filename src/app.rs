//! Top-level application state and event loop wiring.

use crate::curl::{looks_like_curl, parse_curl};
use crate::http::{self, AppSettings, HistoryEntry, RequestSpec, ResponseData, SavedRequest};
use crate::jsonpath::CopyMode;
use crate::storage::Database;
use crate::ui::{
    request_pane::{self, RequestPaneState},
    response_pane::{self, ResponsePaneState},
    shortcuts_help, theme, toast,
};
use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

enum HttpMsg {
    Done(ResponseData),
}

struct Tab {
    name: String,
    spec: RequestSpec,
    response: Option<ResponseData>,
    request_ui: RequestPaneState,
    response_ui: ResponsePaneState,
    dirty: bool,
}

impl Tab {
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            spec: RequestSpec::default(),
            response: None,
            request_ui: RequestPaneState::default(),
            response_ui: ResponsePaneState::default(),
            dirty: false,
        }
    }
}

pub struct BluebirdApp {
    tabs: Vec<Tab>,
    active: usize,
    db: Database,
    settings: AppSettings,
    history: Vec<HistoryEntry>,
    saved: Vec<SavedRequest>,
    show_sidebar: bool,
    show_shortcuts: bool,
    shortcut_filter: String,
    toasts: toast::ToastHost,
    // HTTP
    sending: bool,
    cancel_flag: Arc<AtomicBool>,
    http_tx: Sender<HttpMsg>,
    http_rx: Receiver<HttpMsg>,
    // Curl import
    curl_banner: Option<CurlBanner>,
    pending_curl: Option<RequestSpec>,
    show_replace_prompt: bool,
    prev_url: String,
    // Save dialog
    show_save: bool,
    save_name: String,
}

enum CurlBanner {
    Success,
    Partial(Vec<String>),
    Failure(String),
}

impl BluebirdApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let _ = env_logger::try_init();

        let db = Database::open_default().unwrap_or_else(|e| {
            eprintln!("DB open failed: {e}; using temp");
            let p = std::env::temp_dir().join("bluebird.db");
            Database::open(&p).expect("temp db")
        });
        let settings = db.get_settings().unwrap_or_default();
        let _ = db.prune_history(settings.history_retention_days);
        let history = db.list_history(100).unwrap_or_default();
        let saved = db.list_saved().unwrap_or_default();

        let (http_tx, http_rx) = mpsc::channel();

        let mut tab = Tab::new("Request 1");
        tab.response_ui.copy_mode = CopyMode::from_str(&settings.copy_mode);
        tab.spec.timeout_ms = settings.timeout_ms;
        tab.spec.follow_redirects = settings.follow_redirects;
        tab.spec.max_redirects = settings.max_redirects;

        Self {
            tabs: vec![tab],
            active: 0,
            db,
            settings,
            history,
            saved,
            show_sidebar: true,
            show_shortcuts: false,
            shortcut_filter: String::new(),
            toasts: toast::ToastHost::default(),
            sending: false,
            cancel_flag: Arc::new(AtomicBool::new(false)),
            http_tx,
            http_rx,
            curl_banner: None,
            pending_curl: None,
            show_replace_prompt: false,
            prev_url: String::new(),
            show_save: false,
            save_name: String::new(),
        }
    }

    fn active_tab_mut(&mut self) -> &mut Tab {
        &mut self.tabs[self.active]
    }

    fn send_request(&mut self) {
        let tab = &self.tabs[self.active];
        if tab.spec.url.trim().is_empty() {
            self.toasts.error("Invalid URL.");
            return;
        }
        if url::Url::parse(tab.spec.url.trim()).is_err()
            && !looks_like_curl(&tab.spec.url)
        {
            // Allow host without scheme? require scheme for v1
            if !tab.spec.url.contains("://") {
                self.toasts.error("Invalid URL.");
                return;
            }
        }

        self.sending = true;
        self.cancel_flag = Arc::new(AtomicBool::new(false));
        let flag = self.cancel_flag.clone();
        let spec = tab.spec.clone();
        let tx = self.http_tx.clone();
        thread::spawn(move || {
            let result = match http::execute(&spec, flag) {
                Ok(r) => r,
                Err(e) => ResponseData {
                    status: 0,
                    status_text: String::new(),
                    duration_ms: 0,
                    size_bytes: 0,
                    headers: Vec::new(),
                    body_text: String::new(),
                    is_json: false,
                    cancelled: false,
                    error: Some(e.to_string()),
                },
            };
            let _ = tx.send(HttpMsg::Done(result));
        });
    }

    fn cancel_request(&mut self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    fn poll_http(&mut self) {
        while let Ok(msg) = self.http_rx.try_recv() {
            match msg {
                HttpMsg::Done(resp) => {
                    self.sending = false;
                    let status = if resp.cancelled {
                        None
                    } else if resp.error.is_some() {
                        None
                    } else {
                        Some(resp.status)
                    };
                    let dur = if resp.cancelled { None } else { Some(resp.duration_ms) };
                    let method = self.tabs[self.active].spec.method.clone();
                    let url = self.tabs[self.active].spec.url.clone();
                    let _ = self.db.add_history(&method, &url, status, dur);
                    self.history = self.db.list_history(100).unwrap_or_default();
                    self.tabs[self.active].response = Some(resp);
                }
            }
        }
    }

    fn copy_curl(&mut self, ctx: &egui::Context) {
        let redact = self.settings.redact_on_copy;
        if !self.settings.warned_clipboard_secrets {
            self.toasts.warn(
                "Clipboard may contain secrets. Enable 'Redact secrets' in settings if needed.",
            );
            self.settings.warned_clipboard_secrets = true;
            let _ = self.db.save_settings(&self.settings);
        }
        let curl = http::to_curl(&self.tabs[self.active].spec, redact);
        copy_text(&curl, ctx);
        self.toasts.success("Copied as cURL.");
    }

    fn try_import_curl(&mut self) {
        let url = self.tabs[self.active].spec.url.clone();
        if !looks_like_curl(&url) {
            self.curl_banner = None;
            return;
        }
        match parse_curl(&url) {
            Ok(result) => {
                if self.tabs[self.active].dirty
                    || !self.prev_url.is_empty()
                        && !looks_like_curl(&self.prev_url)
                        && self.prev_url != url
                {
                    self.pending_curl = Some(result.spec);
                    // Stash warnings on banner after confirm
                    if result.warnings.is_empty() {
                        self.curl_banner = Some(CurlBanner::Success);
                    } else {
                        self.curl_banner = Some(CurlBanner::Partial(result.warnings));
                    }
                    self.show_replace_prompt = true;
                } else {
                    self.apply_curl(result.spec, result.warnings);
                }
            }
            Err(e) => {
                self.curl_banner = Some(CurlBanner::Failure(e.to_string()));
            }
        }
    }

    fn apply_curl(&mut self, spec: RequestSpec, warnings: Vec<String>) {
        let tab = &mut self.tabs[self.active];
        tab.spec = spec;
        tab.dirty = false;
        tab.response = None;
        if warnings.is_empty() {
            self.curl_banner = Some(CurlBanner::Success);
        } else {
            self.curl_banner = Some(CurlBanner::Partial(warnings));
        }
    }

    fn new_tab(&mut self) {
        let n = self.tabs.len() + 1;
        let mut t = Tab::new(format!("Request {}", n));
        t.response_ui.copy_mode = CopyMode::from_str(&self.settings.copy_mode);
        self.tabs.push(t);
        self.active = self.tabs.len() - 1;
    }

    fn close_tab(&mut self) {
        if self.tabs.len() <= 1 {
            self.tabs[0] = Tab::new("Request 1");
            self.active = 0;
            return;
        }
        self.tabs.remove(self.active);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut send = false;
        let mut cancel = false;
        let mut copy = false;
        let mut new_tab = false;
        let mut close_tab = false;
        let mut focus_url = false;
        let mut toggle_fmt = false;
        let mut help = false;
        let mut sidebar = false;

        ctx.input(|i| {
            let cmd = i.modifiers.command;
            if cmd && i.key_pressed(egui::Key::Enter) {
                send = true;
            }
            if i.key_pressed(egui::Key::Escape) && self.sending {
                cancel = true;
            }
            if cmd && i.modifiers.shift && i.key_pressed(egui::Key::C) {
                copy = true;
            }
            if cmd && i.key_pressed(egui::Key::T) {
                new_tab = true;
            }
            if cmd && i.key_pressed(egui::Key::W) {
                close_tab = true;
            }
            if cmd && i.key_pressed(egui::Key::L) {
                focus_url = true;
            }
            if cmd && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                toggle_fmt = true;
            }
            if cmd && i.key_pressed(egui::Key::Slash) {
                help = true;
            }
            if cmd && i.key_pressed(egui::Key::B) {
                sidebar = true;
            }
        });

        if send && !self.sending {
            self.send_request();
        }
        if cancel {
            self.cancel_request();
        }
        if copy {
            self.copy_curl(ctx);
        }
        if new_tab {
            self.new_tab();
        }
        if close_tab {
            self.close_tab();
        }
        if focus_url {
            self.tabs[self.active].request_ui.focus_url = true;
        }
        if toggle_fmt {
            let v = &mut self.tabs[self.active].response_ui.body_view;
            *v = match *v {
                response_pane::BodyView::Pretty => response_pane::BodyView::Raw,
                response_pane::BodyView::Raw => response_pane::BodyView::Pretty,
            };
        }
        if help {
            self.show_shortcuts = true;
        }
        if sidebar {
            self.show_sidebar = !self.show_sidebar;
        }
    }
}

impl eframe::App for BluebirdApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_http();
        self.handle_shortcuts(ctx);

        // Top menu
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New tab").clicked() {
                        self.new_tab();
                        ui.close_menu();
                    }
                    if ui.button("Save request…").clicked() {
                        self.show_save = true;
                        self.save_name = self.tabs[self.active].name.clone();
                        ui.close_menu();
                    }
                    if ui.button("Clear history").clicked() {
                        let _ = self.db.clear_history();
                        self.history.clear();
                        self.toasts.info("History cleared.");
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui
                        .checkbox(&mut self.settings.redact_on_copy, "Redact secrets when copying")
                        .changed()
                    {
                        let _ = self.db.save_settings(&self.settings);
                    }
                    ui.label("Default copy mode:");
                    for m in [CopyMode::Js, CopyMode::JsonPath, CopyMode::Variable] {
                        if ui
                            .selectable_label(
                                self.settings.copy_mode == m.as_str(),
                                m.label(),
                            )
                            .clicked()
                        {
                            self.settings.copy_mode = m.as_str().to_string();
                            let _ = self.db.save_settings(&self.settings);
                        }
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("Keyboard shortcuts").clicked() {
                        self.show_shortcuts = true;
                        ui.close_menu();
                    }
                });
                if ui.button("?").on_hover_text("Shortcuts (Ctrl+/)").clicked() {
                    self.show_shortcuts = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("Bluebird")
                            .color(theme::ACCENT)
                            .strong(),
                    );
                });
            });
        });

        // Sidebar
        if self.show_sidebar {
            egui::SidePanel::left("sidebar")
                .default_width(240.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("History");
                    if ui.small_button("Clear").clicked() {
                        let _ = self.db.clear_history();
                        self.history.clear();
                    }
                    egui::ScrollArea::vertical()
                        .id_salt("hist")
                        .max_height(ui.available_height() * 0.5)
                        .show(ui, |ui| {
                            for h in &self.history.clone() {
                                let label = format!("{} {} {}", h.method, h.status.map(|s| s.to_string()).unwrap_or_else(|| "-".into()), truncate(&h.url, 40));
                                if ui.selectable_label(false, label).clicked() {
                                    self.tabs[self.active].spec.method = h.method.clone();
                                    self.tabs[self.active].spec.url = h.url.clone();
                                }
                            }
                        });
                    ui.separator();
                    ui.heading("Saved");
                    egui::ScrollArea::vertical().id_salt("saved").show(ui, |ui| {
                        let saved = self.saved.clone();
                        for s in &saved {
                            ui.horizontal(|ui| {
                                if ui.selectable_label(false, &s.name).clicked() {
                                    if let Ok(spec) =
                                        serde_json::from_str::<RequestSpec>(&s.spec_json)
                                    {
                                        self.tabs[self.active].spec = spec;
                                        self.tabs[self.active].name = s.name.clone();
                                    }
                                }
                                if ui.small_button("×").clicked() {
                                    let _ = self.db.delete_saved(&s.id);
                                    self.saved = self.db.list_saved().unwrap_or_default();
                                }
                            });
                        }
                    });
                });
        }

        // Tabs bar
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for i in 0..self.tabs.len() {
                    let name = self.tabs[i].name.clone();
                    if ui.selectable_label(self.active == i, name).clicked() {
                        self.active = i;
                    }
                }
                if ui.small_button("+").on_hover_text("New tab (Ctrl+T)").clicked() {
                    self.new_tab();
                }
            });
        });

        // Central: request + response split
        egui::CentralPanel::default().show(ctx, |ui| {
            // Clone banner payload so we never hold an immutable borrow while dismissing.
            let banner_kind: Option<(u8, String)> =
                self.curl_banner.as_ref().map(|b| match b {
                    CurlBanner::Success => (0u8, String::new()),
                    CurlBanner::Partial(w) => (1u8, w.join("; ")),
                    CurlBanner::Failure(m) => (2u8, m.clone()),
                });
            let mut dismiss_banner = false;
            if let Some((kind, msg)) = banner_kind {
                ui.horizontal(|ui| {
                    match kind {
                        0 => {
                            ui.colored_label(theme::SUCCESS, "Imported from curl command.");
                        }
                        1 => {
                            ui.colored_label(
                                theme::WARNING,
                                format!("Imported with warnings: {}", msg),
                            );
                        }
                        _ => {
                            ui.colored_label(
                                theme::ERROR,
                                format!("Could not parse curl command. ({})", msg),
                            );
                        }
                    }
                    if ui.small_button("Dismiss").clicked() {
                        dismiss_banner = true;
                    }
                });
            }
            if dismiss_banner {
                self.curl_banner = None;
            }

            let available = ui.available_height();
            let req_h = available * 0.45;

            // Request pane
            ui.allocate_ui(egui::vec2(ui.available_width(), req_h), |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("req_scroll")
                    .show(ui, |ui| {
                        let sending = self.sending;
                        let actions = {
                            let tab = &mut self.tabs[self.active];
                            request_pane::show(ui, &mut tab.spec, &mut tab.request_ui, sending)
                        };
                        if actions.url_changed {
                            self.tabs[self.active].dirty = true;
                            // Debounce-ish: import when looks like curl
                            let url = self.tabs[self.active].spec.url.clone();
                            if looks_like_curl(&url) && url != self.prev_url {
                                self.try_import_curl();
                            }
                            self.prev_url = url;
                        }
                        if actions.send {
                            self.send_request();
                        }
                        if actions.cancel {
                            self.cancel_request();
                        }
                        if actions.copy_curl {
                            self.copy_curl(ctx);
                        }
                    });
            });

            ui.separator();

            // Response pane
            let clicked = {
                let tab = &mut self.tabs[self.active];
                response_pane::show(
                    ui,
                    tab.response.as_ref(),
                    &mut tab.response_ui,
                    self.sending,
                )
            };
            if let Some(path) = clicked {
                copy_text(&path, ctx);
                self.toasts.success(format!("Copied: {}", path));
            }
        });

        // Dialogs — extract data before closures to avoid overlapping borrows.
        if self.show_replace_prompt {
            let mut do_replace = false;
            let mut do_cancel = false;
            egui::Window::new("Replace current request?")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Replace current request with imported curl?");
                    ui.horizontal(|ui| {
                        if ui.button("Replace").clicked() {
                            do_replace = true;
                        }
                        if ui.button("Cancel").clicked() {
                            do_cancel = true;
                        }
                    });
                });
            if do_replace {
                let warnings = match &self.curl_banner {
                    Some(CurlBanner::Partial(w)) => w.clone(),
                    _ => Vec::new(),
                };
                if let Some(spec) = self.pending_curl.take() {
                    self.apply_curl(spec, warnings);
                }
                self.show_replace_prompt = false;
            } else if do_cancel {
                self.pending_curl = None;
                self.show_replace_prompt = false;
                self.curl_banner = None;
            }
        }

        if self.show_save {
            egui::Window::new("Save request")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut self.save_name);
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            let name = self.save_name.trim();
                            if !name.is_empty() {
                                let spec = self.tabs[self.active].spec.clone();
                                if let Ok(id) = self.db.save_request(name, &spec) {
                                    let _ = id;
                                    self.saved = self.db.list_saved().unwrap_or_default();
                                    self.toasts.success("Request saved.");
                                    self.show_save = false;
                                }
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save = false;
                        }
                    });
                });
        }

        shortcuts_help::show_dialog(ctx, &mut self.show_shortcuts, &mut self.shortcut_filter);
        self.toasts.show(ctx);
    }
}

fn copy_text(text: &str, ctx: &egui::Context) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text);
    } else {
        ctx.copy_text(text.to_string());
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max).collect();
        format!("{}…", t)
    }
}
