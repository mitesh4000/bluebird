//! Response inspection pane.

use crate::http::models::ResponseData;
use crate::jsonpath::CopyMode;
use crate::ui::json_tree;
use crate::ui::theme;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ResponseTab {
    Body,
    Headers,
    Cookies,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BodyView {
    Pretty,
    Raw,
}

pub struct ResponsePaneState {
    pub tab: ResponseTab,
    pub body_view: BodyView,
    pub search: String,
    pub copy_mode: CopyMode,
}

impl Default for ResponsePaneState {
    fn default() -> Self {
        Self {
            tab: ResponseTab::Body,
            body_view: BodyView::Pretty,
            search: String::new(),
            copy_mode: CopyMode::Js,
        }
    }
}

/// Returns path string if user clicked a JSON node.
pub fn show(
    ui: &mut egui::Ui,
    response: Option<&ResponseData>,
    state: &mut ResponsePaneState,
    sending: bool,
) -> Option<String> {
    let mut clicked_path = None;

    if sending {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Sending…");
        });
        return None;
    }

    let Some(resp) = response else {
        ui.label(
            egui::RichText::new("Send a request to see the response.")
                .color(theme::TEXT_SECONDARY),
        );
        return None;
    };

    if resp.cancelled {
        ui.colored_label(theme::WARNING, "Request cancelled.");
        return None;
    }

    if let Some(err) = &resp.error {
        ui.colored_label(theme::ERROR, format!("Network error: {}", err));
        return None;
    }

    // Status line
    ui.horizontal(|ui| {
        let color = theme::status_color(resp.status);
        let icon = if (200..300).contains(&resp.status) {
            "✓"
        } else if resp.status >= 400 {
            "✗"
        } else {
            "•"
        };
        ui.colored_label(
            color,
            format!("{} {} {}", icon, resp.status, resp.status_text),
        );
        ui.label(format!("{} ms", resp.duration_ms));
        ui.label(format!("{} bytes", resp.size_bytes));
    });

    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, ResponseTab::Body, "Body");
        ui.selectable_value(&mut state.tab, ResponseTab::Headers, "Headers");
        ui.selectable_value(&mut state.tab, ResponseTab::Cookies, "Cookies");
    });
    ui.separator();

    match state.tab {
        ResponseTab::Body => {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.body_view, BodyView::Pretty, "Pretty");
                ui.selectable_value(&mut state.body_view, BodyView::Raw, "Raw");
                ui.separator();
                ui.label("Path mode:");
                for m in [CopyMode::Js, CopyMode::JsonPath, CopyMode::Variable] {
                    ui.selectable_value(&mut state.copy_mode, m, m.label());
                }
                ui.separator();
                ui.label("Search:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.search)
                        .desired_width(160.0)
                        .hint_text("Filter…"),
                );
            });

            egui::ScrollArea::vertical()
                .id_salt("resp_body")
                .max_height(ui.available_height() - 8.0)
                .show(ui, |ui| {
                    if state.body_view == BodyView::Raw || !resp.is_json {
                        if !resp.is_json && state.body_view == BodyView::Pretty {
                            ui.colored_label(
                                theme::WARNING,
                                "Response is not valid JSON. Path copy disabled.",
                            );
                        }
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&resp.body_text)
                                    .monospace()
                                    .color(theme::TEXT_PRIMARY),
                            )
                            .selectable(true),
                        );
                    } else {
                        match serde_json::from_str::<serde_json::Value>(&resp.body_text) {
                            Ok(value) => {
                                clicked_path = json_tree::show_json_tree(
                                    ui,
                                    &value,
                                    state.copy_mode,
                                    &state.search,
                                );
                            }
                            Err(_) => {
                                ui.colored_label(
                                    theme::WARNING,
                                    "Response is not valid JSON. Path copy disabled.",
                                );
                                ui.monospace(&resp.body_text);
                            }
                        }
                    }
                });
        }
        ResponseTab::Headers => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (k, v) in &resp.headers {
                    ui.horizontal(|ui| {
                        ui.colored_label(theme::ACCENT, k);
                        ui.label(":");
                        ui.monospace(v);
                    });
                }
            });
        }
        ResponseTab::Cookies => {
            let cookies: Vec<_> = resp
                .headers
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
                .collect();
            if cookies.is_empty() {
                ui.label(
                    egui::RichText::new("No Set-Cookie headers.")
                        .color(theme::TEXT_SECONDARY),
                );
            } else {
                for (_, v) in cookies {
                    ui.monospace(v);
                }
            }
        }
    }

    clicked_path
}
