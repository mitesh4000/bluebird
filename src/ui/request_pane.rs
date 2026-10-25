//! Request composer pane.

use crate::http::models::*;
use crate::ui::{icons, theme};
use eframe::egui;

const METHOD_WIDTH: f32 = 100.0;
const SEND_WIDTH: f32 = 104.0;
const COPY_WIDTH: f32 = 140.0;
const SAVE_WIDTH: f32 = 92.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RequestTab {
    Headers,
    Body,
    Auth,
}

pub struct RequestPaneState {
    pub tab: RequestTab,
    pub focus_url: bool,
}

impl Default for RequestPaneState {
    fn default() -> Self {
        Self {
            tab: RequestTab::Headers,
            focus_url: false,
        }
    }
}

pub struct RequestPaneActions {
    pub send: bool,
    pub cancel: bool,
    pub copy_curl: bool,
    pub url_changed: bool,
    pub save: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    spec: &mut RequestSpec,
    state: &mut RequestPaneState,
    sending: bool,
) -> RequestPaneActions {
    let mut actions = RequestPaneActions {
        send: false,
        cancel: false,
        copy_curl: false,
        url_changed: false,
        save: false,
    };

    ui.horizontal(|ui| {
        let height = theme::CONTROL_HEIGHT;
        let gap = ui.spacing().item_spacing.x;

        egui::ComboBox::from_id_salt("method")
            .selected_text(&spec.method)
            .width(METHOD_WIDTH)
            .show_ui(ui, |ui| {
                for m in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                    ui.selectable_value(&mut spec.method, m.to_string(), m);
                }
            });

        // Reserve the button widths first, then give the URL input the rest.
        // Both inputs and buttons use CONTROL_HEIGHT so the row lines up.
        let reserved = SEND_WIDTH + COPY_WIDTH + SAVE_WIDTH + gap * 3.0;
        let url_width = (ui.available_width() - reserved).max(160.0);
        let url_edit = ui.add_sized(
            [url_width, height],
            egui::TextEdit::singleline(&mut spec.url)
                .hint_text("https://api.example.com  ·  or paste a curl command")
                .vertical_align(egui::Align::Center),
        );
        if state.focus_url {
            url_edit.request_focus();
            state.focus_url = false;
        }
        if url_edit.changed() {
            actions.url_changed = true;
        }

        let can_send = !spec.url.trim().is_empty();
        // Single-input workflow: pressing Enter in the URL field sends the
        // request immediately. TextEdit reports Enter as lost_focus().
        let enter_pressed =
            url_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if enter_pressed && can_send && !sending {
            actions.send = true;
        }

        if sending {
            if ui
                .add_sized(
                    [SEND_WIDTH, height],
                    egui::Button::new(icons::with_label(icons::CANCEL, "Cancel")),
                )
                .on_hover_text("Cancel (Esc)")
                .clicked()
            {
                actions.cancel = true;
            }
        } else if ui
            .add_enabled(
                can_send,
                egui::Button::new(icons::with_label(icons::SEND, "Send"))
                    .min_size(egui::vec2(SEND_WIDTH, height)),
            )
            .on_hover_text("Send (Enter or Ctrl+Enter)")
            .clicked()
        {
            actions.send = true;
        }

        if ui
            .add_sized(
                [COPY_WIDTH, height],
                egui::Button::new(icons::with_label(icons::COPY, "Copy cURL")),
            )
            .on_hover_text("Copy as cURL (Ctrl+Shift+C)")
            .clicked()
        {
            actions.copy_curl = true;
        }
        if ui
            .add_sized(
                [SAVE_WIDTH, height],
                egui::Button::new(icons::with_label(icons::SAVE, "Save")),
            )
            .clicked()
        {
            actions.save = true;
        }
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut spec.ssl_verify, "Verify SSL");
        if !spec.ssl_verify {
            ui.colored_label(
                theme::WARNING,
                icons::with_label(icons::WARNING, "insecure"),
            );
        }
        ui.label("Timeout (ms):");
        ui.add(
            egui::DragValue::new(&mut spec.timeout_ms)
                .range(1000..=300_000)
                .speed(100),
        );
        ui.checkbox(&mut spec.follow_redirects, "Follow redirects");
    });

    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, RequestTab::Headers, "Headers");
        ui.selectable_value(&mut state.tab, RequestTab::Body, "Body");
        ui.selectable_value(&mut state.tab, RequestTab::Auth, "Auth");
    });
    ui.separator();

    match state.tab {
        RequestTab::Headers => show_headers(ui, &mut spec.headers),
        RequestTab::Body => show_body(ui, &mut spec.body),
        RequestTab::Auth => show_auth(ui, &mut spec.auth),
    }

    actions
}

fn show_headers(ui: &mut egui::Ui, headers: &mut Vec<HeaderKv>) {
    let mut remove = None;
    for (i, h) in headers.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut h.enabled, "");
            text_input(ui, &mut h.key, 160.0, "Header");
            ui.label(":");
            text_input(ui, &mut h.value, 280.0, "Value");
            if ui
                .add(egui::Button::new(icons::CLOSE).frame(false))
                .on_hover_text("Remove header")
                .clicked()
            {
                remove = Some(i);
            }
        });
        if !h.key.is_empty() {
            let lower = h.key.to_lowercase();
            let suggestions: Vec<&&str> = COMMON_HEADERS
                .iter()
                .filter(|c| c.to_lowercase().starts_with(&lower) && c.to_lowercase() != lower)
                .take(3)
                .collect();
            if !suggestions.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Suggestions:")
                            .color(theme::TEXT_SECONDARY)
                            .small(),
                    );
                    for s in suggestions {
                        if ui.small_button(*s).clicked() {
                            h.key = (*s).to_string();
                        }
                    }
                });
            }
        }
    }
    if let Some(i) = remove {
        headers.remove(i);
    }
    if ui
        .button(icons::with_label(icons::ADD, "Add header"))
        .clicked()
    {
        headers.push(HeaderKv::new("", ""));
    }
}

fn show_body(ui: &mut egui::Ui, body: &mut Body) {
    ui.horizontal(|ui| {
        ui.label("Type:");
        for (kind, label) in [
            (BodyKind::None, "None"),
            (BodyKind::Raw, "Raw"),
            (BodyKind::FormUrlEncoded, "Form URL-encoded"),
            (BodyKind::Multipart, "Multipart"),
            (BodyKind::Binary, "Binary"),
        ] {
            ui.selectable_value(&mut body.kind, kind, label);
        }
    });

    match body.kind {
        BodyKind::None => {
            ui.label(egui::RichText::new("No body").color(theme::TEXT_SECONDARY));
        }
        BodyKind::Raw => {
            ui.horizontal(|ui| {
                ui.label("Content-Type:");
                for ct in [
                    "application/json",
                    "application/xml",
                    "text/html",
                    "text/plain",
                    "application/javascript",
                ] {
                    ui.selectable_value(&mut body.raw_content_type, ct.to_string(), ct);
                }
            });
            ui.add(
                egui::TextEdit::multiline(&mut body.raw_text)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("Request body"),
            );
        }
        BodyKind::FormUrlEncoded => {
            let mut remove = None;
            for (i, (k, v)) in body.form_fields.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    text_input(ui, k, 160.0, "key");
                    ui.label("=");
                    text_input(ui, v, 240.0, "value");
                    if ui
                        .add(egui::Button::new(icons::CLOSE).frame(false))
                        .on_hover_text("Remove field")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                body.form_fields.remove(i);
            }
            if ui
                .button(icons::with_label(icons::ADD, "Add field"))
                .clicked()
            {
                body.form_fields.push((String::new(), String::new()));
            }
        }
        BodyKind::Multipart => {
            let mut remove = None;
            for (i, p) in body.multipart.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    text_input(ui, &mut p.name, 120.0, "name");
                    ui.checkbox(&mut p.is_file, "File");
                    if p.is_file {
                        text_input(ui, &mut p.file_path, 220.0, "path");
                        if ui
                            .button(icons::with_label(icons::BROWSE, "Browse…"))
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                p.file_path = path.display().to_string();
                            }
                        }
                    } else {
                        text_input(ui, &mut p.value, 220.0, "value");
                    }
                    if ui
                        .add(egui::Button::new(icons::CLOSE).frame(false))
                        .on_hover_text("Remove part")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                body.multipart.remove(i);
            }
            if ui
                .button(icons::with_label(icons::ADD, "Add part"))
                .clicked()
            {
                body.multipart.push(FormField {
                    name: String::new(),
                    value: String::new(),
                    is_file: false,
                    file_path: String::new(),
                });
            }
        }
        BodyKind::Binary => {
            ui.horizontal(|ui| {
                text_input(ui, &mut body.binary_path, 320.0, "File path");
                if ui
                    .button(icons::with_label(icons::BROWSE, "Browse…"))
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        body.binary_path = path.display().to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Content-Type:");
                text_input(ui, &mut body.binary_content_type, 260.0, "Content-Type");
            });
        }
    }
}

fn show_auth(ui: &mut egui::Ui, auth: &mut Auth) {
    ui.horizontal(|ui| {
        for (k, label) in [
            (AuthKind::None, "None"),
            (AuthKind::Basic, "Basic"),
            (AuthKind::Bearer, "Bearer"),
            (AuthKind::ApiKey, "API Key"),
        ] {
            ui.selectable_value(&mut auth.kind, k, label);
        }
    });
    match auth.kind {
        AuthKind::None => {
            ui.label(egui::RichText::new("No authentication").color(theme::TEXT_SECONDARY));
        }
        AuthKind::Basic => {
            ui.horizontal(|ui| {
                ui.label("Username:");
                text_input(ui, &mut auth.basic_user, 240.0, "Username");
            });
            ui.horizontal(|ui| {
                ui.label("Password:");
                password_input(ui, &mut auth.basic_pass, 240.0);
            });
        }
        AuthKind::Bearer => {
            ui.horizontal(|ui| {
                ui.label("Token:");
                password_input(ui, &mut auth.bearer_token, 360.0);
            });
        }
        AuthKind::ApiKey => {
            ui.horizontal(|ui| {
                ui.label("Key name:");
                text_input(ui, &mut auth.api_key_name, 240.0, "Key name");
            });
            ui.horizontal(|ui| {
                ui.label("Value:");
                password_input(ui, &mut auth.api_key_value, 280.0);
            });
            ui.horizontal(|ui| {
                ui.label("Add to:");
                ui.selectable_value(&mut auth.api_key_location, ApiKeyLocation::Header, "Header");
                ui.selectable_value(&mut auth.api_key_location, ApiKeyLocation::Query, "Query");
            });
        }
    }
}

/// Single-line input with the shared control height so it aligns with buttons.
fn text_input(
    ui: &mut egui::Ui,
    text: &mut String,
    width: f32,
    hint: &str,
) -> egui::Response {
    ui.add_sized(
        [width, theme::CONTROL_HEIGHT],
        egui::TextEdit::singleline(text)
            .hint_text(hint)
            .vertical_align(egui::Align::Center),
    )
}

fn password_input(ui: &mut egui::Ui, text: &mut String, width: f32) -> egui::Response {
    ui.add_sized(
        [width, theme::CONTROL_HEIGHT],
        egui::TextEdit::singleline(text)
            .password(true)
            .vertical_align(egui::Align::Center),
    )
}
