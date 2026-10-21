//! Request composer pane.

use crate::http::models::*;
use crate::ui::theme;
use eframe::egui;

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
    };

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("method")
            .selected_text(&spec.method)
            .width(100.0)
            .show_ui(ui, |ui| {
                for m in ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"] {
                    ui.selectable_value(&mut spec.method, m.to_string(), m);
                }
            });

        let url_edit = ui.add(
            egui::TextEdit::singleline(&mut spec.url)
                .desired_width(ui.available_width() - 220.0)
                .hint_text("https://api.example.com  ·  or paste a curl command"),
        );
        if state.focus_url {
            url_edit.request_focus();
            state.focus_url = false;
        }
        if url_edit.changed() {
            actions.url_changed = true;
        }

        if sending {
            if ui
                .add(egui::Button::new("Cancel").fill(theme::ERROR))
                .on_hover_text("Esc")
                .clicked()
            {
                actions.cancel = true;
            }
        } else {
            let can_send = !spec.url.trim().is_empty();
            if ui
                .add_enabled(can_send, egui::Button::new("Send").fill(theme::ACCENT))
                .on_hover_text("Ctrl+Enter")
                .clicked()
            {
                actions.send = true;
            }
        }

        if ui
            .button("Copy as cURL")
            .on_hover_text("Ctrl+Shift+C")
            .clicked()
        {
            actions.copy_curl = true;
        }
    });

    ui.horizontal(|ui| {
        ui.checkbox(&mut spec.ssl_verify, "Verify SSL");
        if !spec.ssl_verify {
            ui.colored_label(theme::WARNING, "⚠ insecure");
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
            ui.add(
                egui::TextEdit::singleline(&mut h.key)
                    .desired_width(160.0)
                    .hint_text("Header"),
            );
            ui.label(":");
            ui.add(
                egui::TextEdit::singleline(&mut h.value)
                    .desired_width(280.0)
                    .hint_text("Value"),
            );
            if ui.small_button("×").clicked() {
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
    if ui.button("+ Add header").clicked() {
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
                    ui.add(egui::TextEdit::singleline(k).desired_width(160.0).hint_text("key"));
                    ui.label("=");
                    ui.add(egui::TextEdit::singleline(v).desired_width(240.0).hint_text("value"));
                    if ui.small_button("×").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                body.form_fields.remove(i);
            }
            if ui.button("+ Add field").clicked() {
                body.form_fields.push((String::new(), String::new()));
            }
        }
        BodyKind::Multipart => {
            let mut remove = None;
            for (i, p) in body.multipart.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut p.name)
                            .desired_width(120.0)
                            .hint_text("name"),
                    );
                    ui.checkbox(&mut p.is_file, "File");
                    if p.is_file {
                        ui.add(
                            egui::TextEdit::singleline(&mut p.file_path)
                                .desired_width(220.0)
                                .hint_text("path"),
                        );
                        if ui.small_button("Browse…").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_file() {
                                p.file_path = path.display().to_string();
                            }
                        }
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut p.value)
                                .desired_width(220.0)
                                .hint_text("value"),
                        );
                    }
                    if ui.small_button("×").clicked() {
                        remove = Some(i);
                    }
                });
            }
            if let Some(i) = remove {
                body.multipart.remove(i);
            }
            if ui.button("+ Add part").clicked() {
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
                ui.add(
                    egui::TextEdit::singleline(&mut body.binary_path)
                        .desired_width(320.0)
                        .hint_text("File path"),
                );
                if ui.button("Browse…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        body.binary_path = path.display().to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Content-Type:");
                ui.text_edit_singleline(&mut body.binary_content_type);
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
                ui.text_edit_singleline(&mut auth.basic_user);
            });
            ui.horizontal(|ui| {
                ui.label("Password:");
                ui.add(egui::TextEdit::singleline(&mut auth.basic_pass).password(true));
            });
        }
        AuthKind::Bearer => {
            ui.horizontal(|ui| {
                ui.label("Token:");
                ui.add(
                    egui::TextEdit::singleline(&mut auth.bearer_token)
                        .desired_width(360.0)
                        .password(true),
                );
            });
        }
        AuthKind::ApiKey => {
            ui.horizontal(|ui| {
                ui.label("Key name:");
                ui.text_edit_singleline(&mut auth.api_key_name);
            });
            ui.horizontal(|ui| {
                ui.label("Value:");
                ui.add(
                    egui::TextEdit::singleline(&mut auth.api_key_value)
                        .desired_width(280.0)
                        .password(true),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Add to:");
                ui.selectable_value(&mut auth.api_key_location, ApiKeyLocation::Header, "Header");
                ui.selectable_value(&mut auth.api_key_location, ApiKeyLocation::Query, "Query");
            });
        }
    }
}
