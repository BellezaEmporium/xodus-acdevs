use crate::app::GameLauncher;
use eframe::egui::{self};

// WIP, BellezaEmporium 2026.
// Part of the Xodus UI & the Xodus Game Launcher, for Linux & macOS.

impl GameLauncher {
    pub(crate) fn render_shell(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let sidebar_width = (rect.width() * 0.24).min(285.0);
        let sidebar_rect = egui::Rect::from_min_max(
            rect.min,
            egui::pos2(rect.left() + sidebar_width, rect.bottom()),
        );
        let content_rect =
            egui::Rect::from_min_max(egui::pos2(sidebar_rect.right(), rect.top()), rect.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(sidebar_rect), |ui| {
            self.sidebar(ui)
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
            self.main_content(ui)
        });
    }
}
