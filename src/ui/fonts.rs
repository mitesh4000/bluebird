//! Custom icon font installation.
//!
//! The Font Awesome Free Solid font is bundled with the binary so the app
//! stays fully offline. Its glyphs live in the Unicode Private Use Area, so
//! appending it as a fallback to the proportional family means the regular UI
//! font is still preferred for prose while symbol glyphs resolve correctly.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

pub const ICON_FONT: &str = "fa-solid";

/// Register the bundled icon font with an egui context.
///
/// Call this once, before the first frame is drawn (typically from the
/// `eframe::CreationContext` inside `eframe::run_native`).
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        ICON_FONT.to_owned(),
        FontData::from_static(include_bytes!("../../assets/fa-solid-900.ttf")),
    );

    // Append (not prepend) so normal text keeps using the default font and
    // only missing glyphs fall through to the icon font.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .push(ICON_FONT.to_owned());

    // Also allow icons in monospace contexts (e.g. status rows).
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .push(ICON_FONT.to_owned());

    ctx.set_fonts(fonts);
}
