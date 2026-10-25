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

/// Shared height for single-line inputs and their adjacent action buttons.
/// Using one token keeps rows visually aligned instead of letting egui size
/// each widget from its own text/padding metrics.
pub const CONTROL_HEIGHT: f32 = 32.0;

pub fn highlight_hover() -> Color32 {
    Color32::from_rgba_unmultiplied(0x2F, 0x81, 0xF7, 36)
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_visuals(Visuals::light());

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = CONTROL_HEIGHT;
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
