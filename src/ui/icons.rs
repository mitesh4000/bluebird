//! Icon glyphs from the bundled Font Awesome Free Solid font.
//!
//! Keep all codepoints in one place so the UI never depends on ad-hoc Unicode
//! symbols that may be missing from the default font.

// Actions
pub const SEND: &str = "\u{f1d8}"; // paper-plane
pub const CANCEL: &str = "\u{f00d}"; // xmark
pub const CLOSE: &str = "\u{f00d}"; // xmark
pub const COPY: &str = "\u{f0c5}"; // copy
pub const SAVE: &str = "\u{f0c7}"; // floppy-disk
pub const ADD: &str = "\u{f067}"; // plus
pub const DELETE: &str = "\u{f2ed}"; // trash-can
pub const BROWSE: &str = "\u{f07c}"; // folder-open
pub const SEARCH: &str = "\u{f002}"; // magnifying-glass
pub const HELP: &str = "\u{f059}"; // circle-question
pub const TERMINAL: &str = "\u{f120}"; // terminal

// Navigation / status
pub const CHEVRON_LEFT: &str = "\u{f053}";
pub const CHEVRON_RIGHT: &str = "\u{f054}";
pub const CHECK: &str = "\u{f00c}";
pub const ERROR: &str = "\u{f00d}";
pub const WARNING: &str = "\u{f071}";
pub const INFO: &str = "\u{f05a}";
pub const DOT: &str = "\u{f111}";

/// Build button/label text with a leading icon and a readable label.
pub fn with_label(glyph: &str, label: &str) -> String {
    format!("{glyph}  {label}")
}

/// Icon-only button text.
pub fn icon(glyph: &str) -> String {
    glyph.to_owned()
}
