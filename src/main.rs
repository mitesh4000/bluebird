//! Bluebird — pure-Rust native desktop HTTP API client.

mod app;
mod curl;
mod http;
mod jsonpath;
mod security;
mod storage;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_title("Bluebird"),
        ..Default::default()
    };

    eframe::run_native(
        "Bluebird",
        options,
        Box::new(|cc| Ok(Box::new(app::BluebirdApp::new(cc)))),
    )
}
