//! Blue-and-black theme tokens and egui style wiring.

use eframe::egui::{self, Color32, Stroke, Visuals};

pub const BG_BASE: Color32 = Color32::from_rgb(0x0B, 0x0F, 0x14);
pub const BG_SURFACE: Color32 = Color32::from_rgb(0x12, 0x18, 0x21);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x1A, 0x22, 0x30);
pub const BORDER: Color32 = Color32::from_rgb(0x24, 0x30, 0x40);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE6, 0xED, 0xF3);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x9D, 0xAA, 0xB8);
pub const ACCENT: Color32 = Color32::from_rgb(0x2F, 0x81, 0xF7);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0x4C, 0x93, 0xF8);
pub const ACCENT_PRESSED: Color32 = Color32::from_rgb(0x1F, 0x6F, 0xE0);
pub const SUCCESS: Color32 = Color32::from_rgb(0x2E, 0xA0, 0x43);
pub const WARNING: Color32 = Color32::from_rgb(0xD2, 0x99, 0x22);
pub const ERROR: Color32 = Color32::from_rgb(0xF8, 0x51, 0x49);
pub fn highlight_hover() -> Color32 {
    Color32::from_rgba_unmultiplied(0x2F, 0x81, 0xF7, 36)
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = BG_BASE;
    visuals.window_fill = BG_SURFACE;
    visuals.extreme_bg_color = BG_BASE;
    visuals.faint_bg_color = BG_ELEVATED;
    visuals.widgets.noninteractive.bg_fill = BG_SURFACE;
    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x22, 0x2C, 0x3C);
    visuals.widgets.active.bg_fill = ACCENT_PRESSED;
    visuals.widgets.open.bg_fill = BG_ELEVATED;
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x2F, 0x81, 0xF7, 80);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_SECONDARY);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    visuals.hyperlink_color = ACCENT;
    visuals.warn_fg_color = WARNING;
    visuals.error_fg_color = ERROR;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    ctx.set_style(style);
}

pub fn status_color(code: u16) -> Color32 {
    if (200..300).contains(&code) {
        SUCCESS
    } else if (400..600).contains(&code) {
        ERROR
    } else if code > 0 {
        WARNING
    } else {
        TEXT_SECONDARY
    }
}
