//! Stacking toasts with auto-dismiss.

use crate::ui::theme;
use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub created: Instant,
    pub kind: ToastKind,
}

#[derive(Clone, Copy)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

pub struct ToastHost {
    toasts: Vec<Toast>,
    ttl: Duration,
}

impl Default for ToastHost {
    fn default() -> Self {
        Self {
            toasts: Vec::new(),
            ttl: Duration::from_millis(2500),
        }
    }
}

impl ToastHost {
    pub fn push(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toasts.push(Toast {
            message: message.into(),
            created: Instant::now(),
            kind,
        });
    }

    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(msg, ToastKind::Info);
    }
    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(msg, ToastKind::Success);
    }
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.push(msg, ToastKind::Warning);
    }
    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(msg, ToastKind::Error);
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.created) < self.ttl);

        let screen = ctx.screen_rect();
        let mut y = screen.bottom() - 48.0;
        for t in self.toasts.iter().rev() {
            let color = match t.kind {
                ToastKind::Info => theme::ACCENT,
                ToastKind::Success => theme::SUCCESS,
                ToastKind::Warning => theme::WARNING,
                ToastKind::Error => theme::ERROR,
            };
            egui::Area::new(egui::Id::new("toast").with(t.created.elapsed().as_millis()))
                .fixed_pos(egui::pos2(screen.right() - 360.0, y - 40.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(theme::BG_ELEVATED)
                        .stroke(egui::Stroke::new(1.0_f32, color))
                        // .corner_radius(6.0)
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.colored_label(theme::TEXT_PRIMARY, &t.message);
                        });
                });
            y -= 48.0;
        }
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
