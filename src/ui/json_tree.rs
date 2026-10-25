//! Collapsible JSON tree with click-to-copy path.

use crate::jsonpath::{build_path, CopyMode, PathSegment};
use crate::ui::theme;
use eframe::egui;
use serde_json::Value;

/// Render a JSON value as an interactive tree.
/// Returns Some(path) when the user clicks a node (caller copies + toasts).
pub fn show_json_tree(
    ui: &mut egui::Ui,
    value: &Value,
    mode: CopyMode,
    search: &str,
) -> Option<String> {
    let mut clicked: Option<String> = None;
    show_node(ui, value, &[], mode, search, &mut clicked, true);
    clicked
}

fn show_node(
    ui: &mut egui::Ui,
    value: &Value,
    path: &[PathSegment],
    mode: CopyMode,
    search: &str,
    clicked: &mut Option<String>,
    is_root: bool,
) {
    match value {
        Value::Object(map) => {
            let count = map.len();
            let id = ui.make_persistent_id(("obj", path_id(path)));
            let mut open = is_root;
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open)
                .show_header(ui, |ui| {
                    let label = if path.is_empty() {
                        format!("{{ {} }}", count)
                    } else {
                        format!("{}  {{ {} }}", path.last().map(|s| s.key.as_str()).unwrap_or(""), count)
                    };
                    let path_str = build_path(path, mode, "res");
                    let mut rich = egui::RichText::new(label.clone()).color(theme::TEXT_SECONDARY);
                    let is_match = !search.is_empty() && (matches_search(&label, search) || matches_search(&path_str, search));
                    if is_match {
                        rich = rich.underline().background_color(egui::Color32::from_rgba_unmultiplied(255, 255, 0, 60));
                    }
                    let resp = ui.add(
                        egui::Label::new(rich)
                            .sense(egui::Sense::click()),
                    );
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        resp.clone().on_hover_text(&path_str);
                    }
                    if resp.clicked() {
                        *clicked = Some(path_str);
                    }
                })
                .body(|ui| {
                    // Virtualize large objects: only draw visible-ish window
                    let keys: Vec<&String> = map.keys().collect();
                    let total = keys.len();
                    let max_draw = if total > 200 { 200 } else { total };
                    for (i, k) in keys.iter().take(max_draw).enumerate() {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSegment::key((*k).clone()));
                        let v = &map[*k];
                        if matches!(v, Value::Object(_) | Value::Array(_)) {
                            let key_path = build_path(&child_path, mode, "res");
                            let key_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("{}:", k))
                                        .color(theme::ACCENT),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if key_resp.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                key_resp.clone().on_hover_text(&key_path);
                            }
                            if key_resp.clicked() {
                                *clicked = Some(key_path);
                            }
                            show_node(ui, v, &child_path, mode, search, clicked, false);
                        } else {
                            ui.horizontal(|ui| {
                                let key_path = build_path(&child_path, mode, "res");
                                let key_resp = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("{}:", k))
                                            .color(theme::ACCENT),
                                    )
                                    .sense(egui::Sense::click()),
                                );
                                if key_resp.hovered() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    key_resp.clone().on_hover_text(&key_path);
                                }
                                if key_resp.clicked() {
                                    *clicked = Some(key_path);
                                }
                                show_primitive(ui, v, &child_path, mode, search, clicked);
                            });
                        }
                        let _ = i;
                    }
                    if total > max_draw {
                        ui.label(
                            egui::RichText::new(format!(
                                "… {} more keys (expand filter / search)",
                                total - max_draw
                            ))
                            .color(theme::TEXT_SECONDARY),
                        );
                    }
                });
            let _ = open;
        }
        Value::Array(arr) => {
            let count = arr.len();
            let id = ui.make_persistent_id(("arr", path_id(path)));
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, is_root)
                .show_header(ui, |ui| {
                    let label = if path.is_empty() {
                        format!("[ {} ]", count)
                    } else {
                        format!(
                            "{}  [ {} ]",
                            path.last().map(|s| s.key.as_str()).unwrap_or(""),
                            count
                        )
                    };
                    let path_str = build_path(path, mode, "res");
                    let mut rich = egui::RichText::new(label.clone()).color(theme::TEXT_SECONDARY);
                    let is_match = !search.is_empty() && (matches_search(&label, search) || matches_search(&path_str, search));
                    if is_match {
                        rich = rich.underline().background_color(egui::Color32::from_rgba_unmultiplied(255, 255, 0, 60));
                    }
                    let resp = ui.add(
                        egui::Label::new(rich)
                            .sense(egui::Sense::click()),
                    );
                    if resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        resp.clone().on_hover_text(&path_str);
                    }
                    if resp.clicked() {
                        *clicked = Some(path_str);
                    }
                })
                .body(|ui| {
                    let total = arr.len();
                    let max_draw = if total > 200 { 200 } else { total };
                    for i in 0..max_draw {
                        let mut child_path = path.to_vec();
                        child_path.push(PathSegment::index(i));
                        let v = &arr[i];
                        if matches!(v, Value::Object(_) | Value::Array(_)) {
                            show_node(ui, v, &child_path, mode, search, clicked, false);
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("[{}]", i))
                                        .color(theme::TEXT_SECONDARY),
                                );
                                show_primitive(ui, v, &child_path, mode, search, clicked);
                            });
                        }
                    }
                    if total > max_draw {
                        ui.label(
                            egui::RichText::new(format!("… {} more items", total - max_draw))
                                .color(theme::TEXT_SECONDARY),
                        );
                    }
                });
        }
        _ => {
            show_primitive(ui, value, path, mode, search, clicked);
        }
    }
}

fn show_value_or_nested(
    ui: &mut egui::Ui,
    value: &Value,
    path: &[PathSegment],
    mode: CopyMode,
    search: &str,
    clicked: &mut Option<String>,
) {
    match value {
        Value::Object(_) | Value::Array(_) => {
            show_node(ui, value, path, mode, search, clicked, false);
        }
        _ => show_primitive(ui, value, path, mode, search, clicked),
    }
}

fn show_primitive(
    ui: &mut egui::Ui,
    value: &Value,
    path: &[PathSegment],
    mode: CopyMode,
    search: &str,
    clicked: &mut Option<String>,
) {
    let (text, color) = match value {
        Value::Null => ("null".to_string(), theme::TEXT_SECONDARY),
        Value::Bool(b) => (b.to_string(), theme::WARNING),
        Value::Number(n) => (n.to_string(), theme::SUCCESS),
        Value::String(s) => (format!("\"{}\"", s), COLOR_STRING),
        _ => (value.to_string(), theme::TEXT_PRIMARY),
    };

    let path_str = build_path(path, mode, "res");
    let is_match = matches_search(&text, search) || matches_search(&path_str, search);
    let mut rich = egui::RichText::new(text).color(color).monospace();
    if !search.is_empty() && is_match {
        rich = rich.underline().background_color(egui::Color32::from_rgba_unmultiplied(255, 255, 0, 60));
    } else if !search.is_empty() && !is_match {
        // dim non-matching for better visibility
        rich = rich.color(egui::Color32::from_gray(160));
    }
    let resp = ui.add(
        egui::Label::new(rich)
            .sense(egui::Sense::click()),
    );
    if resp.hovered() {
        ui.painter().rect_filled(
            resp.rect.expand(2.0),
            3.0,
            theme::highlight_hover(),
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        resp.clone().on_hover_text(format!("Copy: {}", path_str));
    }
    if resp.clicked() || (resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
        *clicked = Some(path_str);
    }
}

fn path_id(path: &[PathSegment]) -> String {
    path.iter()
        .map(|s| {
            if s.is_index {
                format!("[{}]", s.key)
            } else {
                s.key.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn matches_search(text: &str, search: &str) -> bool {
    if search.is_empty() {
        return true;
    }
    text.to_lowercase().contains(&search.to_lowercase())
}

const COLOR_STRING: egui::Color32 = egui::Color32::from_rgb(0xCE, 0x91, 0x78);
