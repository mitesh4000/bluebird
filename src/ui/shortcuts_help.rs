//! Searchable keyboard shortcuts dialog.

use crate::ui::theme;
use eframe::egui;

pub struct Shortcut {
    pub action: &'static str,
    pub win: &'static str,
    pub mac: &'static str,
}

pub const SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        action: "Send request",
        win: "Ctrl+Enter",
        mac: "Cmd+Enter",
    },
    Shortcut {
        action: "Cancel request",
        win: "Esc",
        mac: "Esc",
    },
    Shortcut {
        action: "Copy as cURL",
        win: "Ctrl+Shift+C",
        mac: "Cmd+Shift+C",
    },
    Shortcut {
        action: "New tab",
        win: "Ctrl+T",
        mac: "Cmd+T",
    },
    Shortcut {
        action: "Close tab",
        win: "Ctrl+W",
        mac: "Cmd+W",
    },
    Shortcut {
        action: "Focus URL",
        win: "Ctrl+L",
        mac: "Cmd+L",
    },
    Shortcut {
        action: "Toggle Pretty/Raw body",
        win: "Ctrl+Shift+F",
        mac: "Cmd+Shift+F",
    },
    Shortcut {
        action: "Copy focused JSON path",
        win: "Enter",
        mac: "Enter",
    },
    Shortcut {
        action: "Shortcuts help",
        win: "Ctrl+/",
        mac: "Cmd+/",
    },
    Shortcut {
        action: "Toggle history sidebar",
        win: "Ctrl+B",
        mac: "Cmd+B",
    },
];

pub fn show_dialog(ctx: &egui::Context, open: &mut bool, filter: &mut String) {
    if !*open {
        return;
    }
    egui::Window::new("Keyboard shortcuts")
        .open(open)
        .collapsible(false)
        .resizable(true)
        .default_width(420.0)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(filter);
            });
            ui.separator();
            let q = filter.to_lowercase();
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(3)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.strong("Action");
                        ui.strong("Win/Linux");
                        ui.strong("macOS");
                        ui.end_row();
                        for s in SHORTCUTS {
                            if !q.is_empty()
                                && !s.action.to_lowercase().contains(&q)
                                && !s.win.to_lowercase().contains(&q)
                                && !s.mac.to_lowercase().contains(&q)
                            {
                                continue;
                            }
                            ui.label(s.action);
                            ui.colored_label(theme::ACCENT, s.win);
                            ui.colored_label(theme::ACCENT, s.mac);
                            ui.end_row();
                        }
                    });
            });
        });
}
